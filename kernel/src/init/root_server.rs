use super::pm::BumpAllocator;
use shared::elf::def::{Elf64Hdr, ProgramFlags};
use shared::elf::ProgramMapper;

use crate::address::KernelVAddress;
use crate::address::VirtAddr;
use crate::address::PAGE_SIZE;
use crate::capability::cnode::CNodeCap;
use crate::capability::page_table::PageCap;
use crate::capability::page_table::PageTableCap;
use crate::capability::tcb::TCBCap;
use crate::capability::untyped::UntypedCap;
use crate::capability::Capability;
use crate::common::{align_up, ErrKind};
use crate::object::page_table::{Page, PAGE_R, PAGE_U, PAGE_W, PAGE_X};
use crate::object::CNode;
use crate::object::CNodeEntry;
use crate::object::PageTable;
use crate::object::ThreadControlBlock;
use crate::object::ThreadInfo;

use crate::riscv::SSTATUS_SPIE;
use crate::KernelError;
use core::cmp::min;
use core::mem::MaybeUninit;
use core::ptr;

pub const ROOT_TCB_IDX: usize = 1;
pub const ROOT_CNODE_IDX: usize = 2;
pub const ROOT_VSPACE_IDX: usize = 3;
pub const ROOT_IPC_BUFFER: usize = 4;
pub const ROOT_BOOT_INFO_PAGE: usize = 5;
pub const ROOT_CNODE_ENTRY_NUM_BITS: usize = 18; // 2^18

pub(in crate::init) struct RootServerMemory<'a> {
    cnode: &'a mut MaybeUninit<CNode>,
    vspace: &'a mut MaybeUninit<PageTable>,
    tcb: &'a mut MaybeUninit<ThreadControlBlock>,
    ipc_buf: &'a mut MaybeUninit<Page>,
    boot_frame: &'a mut MaybeUninit<Page>,
}

impl<'a> RootServerMemory<'a> {
    fn alloc_obj<T>(
        bump_allocator: &mut BumpAllocator,
        user_size: usize,
    ) -> &'a mut MaybeUninit<T::KernelObject>
    where
        T: Capability,
    {
        // easiest way to care align.
        let start_address = bump_allocator
            .allocate_pages(align_up(T::get_object_size(user_size), PAGE_SIZE) / PAGE_SIZE)
            .into();
        let cnode_ptr = <KernelVAddress as Into<*mut T::KernelObject>>::into(start_address);
        unsafe { cnode_ptr.as_uninit_mut().unwrap() }
    }

    pub fn init_with_uninit(bump_allocator: &mut BumpAllocator) -> Self {
        let cnode = Self::alloc_obj::<CNodeCap>(bump_allocator, ROOT_CNODE_ENTRY_NUM_BITS);
        let vspace = Self::alloc_obj::<PageTableCap>(bump_allocator, 0);
        let tcb = Self::alloc_obj::<TCBCap>(bump_allocator, 0);
        let ipc_buf = Self::alloc_obj::<PageCap>(bump_allocator, 0);
        let boot_frame = Self::alloc_obj::<PageCap>(bump_allocator, 0);
        Self {
            cnode,
            vspace,
            tcb,
            ipc_buf,
            boot_frame,
        }
    }

    pub fn create_root_cnode(&mut self) -> CNodeCap {
        let cnode = self.cnode.write(CNode::new());
        let vaddr = (cnode as *const CNode).into();
        let cap_dep_val = CNodeCap::create_cap_dep_val(vaddr, ROOT_CNODE_ENTRY_NUM_BITS);
        let cap_type = CNodeCap::CAP_TYPE;
        let cap = CNodeCap::new(cap_type, vaddr.into(), cap_dep_val as u64);
        let mut cnode_cap = cap.replicate();
        cnode_cap
            .write_slot(cap.replicate(), ROOT_CNODE_IDX)
            .unwrap();
        cap
    }

    /// create address space of initial server.
    pub fn create_address_space(
        &mut self,
        cnode_cap: &mut CNodeCap,
        elf_header: *const Elf64Hdr,
        root_rsc_mgr: &mut RootServerResourceManager,
    ) -> (PageTableCap, VirtAddr) {
        let root_page_table = self.vspace.write(PageTable::new());

        let vaddr = (root_page_table as *const PageTable).into();
        let mut cap = PageTableCap::init(vaddr, 0);
        cap.make_as_root().unwrap();
        cnode_cap
            .write_slot(cap.replicate(), ROOT_VSPACE_IDX)
            .unwrap();
        let mut mapper = RootServerElfMapper::new(root_rsc_mgr, &mut cap, cnode_cap);
        unsafe {
            (*elf_header).map_self(&mut mapper).unwrap();
        }
        let max_vaddr = mapper.max_vaddr_of_elf();
        (cap, max_vaddr)
    }

    /// create ipc buffer frame
    pub fn create_ipc_buf_frame(
        &mut self,
        cnode_cap: &mut CNodeCap,
        vspace_cap: &mut PageTableCap,
        max_vaddr: VirtAddr,
        bootstage_mbr: &mut RootServerResourceManager,
    ) -> PageCap {
        let ipc_buf_frame = self.ipc_buf.write(Page::new());
        let vaddr = (ipc_buf_frame as *const Page).into();
        let flags = PAGE_R | PAGE_W | PAGE_U;
        let page_cap = create_mapped_page_cap(
            cnode_cap,
            vspace_cap,
            bootstage_mbr,
            vaddr,
            max_vaddr.add(PAGE_SIZE),
            flags,
        );
        cnode_cap.write_slot(page_cap, ROOT_IPC_BUFFER).unwrap();
        page_cap
    }

    pub fn create_boot_info_frame(
        &mut self,
        cnode_cap: &mut CNodeCap,
        vspace_cap: &mut PageTableCap,
        max_vaddr: VirtAddr,
        bootstage_mbr: &mut RootServerResourceManager,
    ) -> (PageCap, KernelVAddress) {
        let boot_page = self.boot_frame.write(Page::new());
        let vaddr = (boot_page as *const Page).into();
        let flags = PAGE_R | PAGE_U;
        let page_cap = create_mapped_page_cap(
            cnode_cap,
            vspace_cap,
            bootstage_mbr,
            vaddr,
            max_vaddr.add(PAGE_SIZE),
            flags,
        );
        cnode_cap.write_slot(page_cap, ROOT_BOOT_INFO_PAGE).unwrap();
        (page_cap, vaddr)
    }

    pub fn create_root_tcb(
        &mut self,
        cnode_cap: &mut CNodeCap,
        vspace_cap: &mut PageTableCap,
        ipc_buf_cap: &mut PageCap,
        entry_point: VirtAddr,
    ) -> TCBCap {
        let tcb = self
            .tcb
            .write(ThreadControlBlock::new(ThreadInfo::default()));
        tcb.registers.sstatus = SSTATUS_SPIE;
        tcb.registers.sepc = entry_point.into();

        // insert cnode_cap into tcb cnode_cap
        let mut new_entry = CNodeEntry::new_with_rawcap(cnode_cap.replicate());
        new_entry.insert(
            cnode_cap
                .lookup_entry_mut_one_level(ROOT_CNODE_IDX)
                .unwrap()
                .as_mut()
                .unwrap(),
        );
        tcb.root_cnode = Some(new_entry);
        // insert vspace cap into tcb vspace
        let mut new_entry = CNodeEntry::new_with_rawcap(vspace_cap.replicate());
        new_entry.insert(
            cnode_cap
                .lookup_entry_mut_one_level(ROOT_VSPACE_IDX)
                .unwrap()
                .as_mut()
                .unwrap(),
        );
        tcb.vspace = Some(new_entry);
        let mut new_entry = CNodeEntry::new_with_rawcap(ipc_buf_cap.replicate());
        new_entry.insert(
            cnode_cap
                .lookup_entry_mut_one_level(ROOT_IPC_BUFFER)
                .unwrap()
                .as_mut()
                .unwrap(),
        );
        tcb.ipc_buffer = Some(new_entry);

        let cap = TCBCap::init((tcb as *const ThreadControlBlock).into(), 0);
        cnode_cap.write_slot(cap.replicate(), ROOT_TCB_IDX).unwrap();
        cap
    }
}

pub(in crate::init) struct RootServerResourceManager {
    bump_allocator: BumpAllocator,
    cnode_satrt_idx: usize,
    cnode_idx_max: usize,
}

impl RootServerResourceManager {
    pub fn new(
        bump_allocator: BumpAllocator,
        cnode_start_idx: usize,
        cnode_idx_max: usize,
    ) -> Self {
        assert!(cnode_start_idx < cnode_idx_max);
        Self {
            bump_allocator,
            cnode_satrt_idx: cnode_start_idx,
            cnode_idx_max,
        }
    }

    pub fn alloc_page(&mut self) -> KernelVAddress {
        self.bump_allocator.allocate_page().into()
    }

    pub fn alloc_cnode_idx(&mut self) -> usize {
        let ret = self.cnode_satrt_idx;
        assert!(ret <= self.cnode_idx_max);
        self.cnode_satrt_idx += 1;
        ret
    }

    pub fn finalize(self) -> UntypedCapGenerator {
        let (start_address, end_address) = self.bump_allocator.end_allocation();
        UntypedCapGenerator {
            start_address: start_address.into(),
            end_address: end_address.into(),
            idx_start: self.cnode_satrt_idx,
            idx_max: self.cnode_idx_max,
        }
    }
}

pub(in crate::init) struct UntypedCapGenerator {
    start_address: KernelVAddress,
    end_address: KernelVAddress,
    idx_start: usize,
    idx_max: usize,
}

impl Iterator for UntypedCapGenerator {
    type Item = (usize, UntypedCap);

    fn next(&mut self) -> Option<Self::Item> {
        if self.start_address.add(4096) >= self.end_address {
            return None;
        }
        assert!(self.idx_max >= self.idx_start);
        let block_size: usize = (self.end_address - self.start_address).into();
        let untyped_cap = UntypedCap::init(self.start_address, block_size);
        let acctual_size = untyped_cap.block_size();
        self.start_address = self.start_address.add(acctual_size);
        let ret = Some((self.idx_start, untyped_cap));
        self.idx_start += 1;
        ret
    }
}

pub struct RootServerElfMapper<'a> {
    max_vaddr: VirtAddr,
    root_rsc_mgr: &'a mut RootServerResourceManager,
    root_table_cap: &'a mut PageTableCap,
    cnode_cap: &'a mut CNodeCap,
}

impl<'a> RootServerElfMapper<'a> {
    pub fn new(
        root_rsc_mgr: &'a mut RootServerResourceManager,
        root_table_cap: &'a mut PageTableCap,
        cnode_cap: &'a mut CNodeCap,
    ) -> Self {
        Self {
            max_vaddr: 0.into(),
            root_rsc_mgr,
            root_table_cap,
            cnode_cap,
        }
    }

    pub fn max_vaddr_of_elf(self) -> VirtAddr {
        self.max_vaddr
    }
}

impl ProgramMapper for RootServerElfMapper<'_> {
    type Flag = usize;
    type Error = KernelError;

    fn get_flags(flag: u32) -> Self::Flag {
        get_flags(flag) | PAGE_U
    }

    fn map_program(
        &mut self,
        vaddr: usize,
        p_start_addr: *const u8,
        p_mem_size: usize,
        p_file_size: usize,
        flags: Self::Flag,
    ) -> Result<(), Self::Error> {
        let page_num = (align_up(p_mem_size, PAGE_SIZE)) / PAGE_SIZE;
        let mut file_sz_rem = p_file_size;
        let vaddr = VirtAddr::new(vaddr);
        for page_idx in 0..page_num {
            let offset = PAGE_SIZE * page_idx;
            let vaddr_n = vaddr.add(offset);
            if self.max_vaddr < vaddr_n {
                self.max_vaddr = vaddr_n;
            }
            let page_addr = self.root_rsc_mgr.alloc_page();
            let page_cap = create_mapped_page_cap(
                self.cnode_cap,
                self.root_table_cap,
                self.root_rsc_mgr,
                page_addr,
                vaddr_n,
                flags,
            );
            self.cnode_cap
                .write_slot(page_cap, self.root_rsc_mgr.alloc_cnode_idx())
                .unwrap();
            if file_sz_rem != 0 {
                let copy_dst = page_cap.get_address_virtual().into();
                let copy_size = min(PAGE_SIZE, file_sz_rem);
                unsafe {
                    let copy_src = p_start_addr.add(offset);
                    ptr::copy::<u8>(copy_src, copy_dst, copy_size);
                }
                file_sz_rem = file_sz_rem.saturating_sub(PAGE_SIZE);
            }
        }
        Ok(())
    }
}

fn create_mapped_page_cap(
    cnode_cap: &mut CNodeCap,
    root_table_cap: &mut PageTableCap,
    bootstage_mbr: &mut RootServerResourceManager,
    paddr: KernelVAddress,
    vaddr_n: VirtAddr,
    flags: usize,
) -> PageCap {
    let mut page_cap = PageCap::init(paddr, 0);
    if let Err(e) = page_cap.map(root_table_cap, vaddr_n, flags) {
        match e.e_kind {
            ErrKind::PageTableNotMappedYet => {
                map_page_tables(cnode_cap, bootstage_mbr, root_table_cap, vaddr_n);
                page_cap.map(root_table_cap, vaddr_n, flags).unwrap();
            }
            ErrKind::VaddressAlreadyMapped => {
                panic!("Should never occur")
            }
            _ => {
                panic!("Unknown Error occured")
            }
        }
    };
    page_cap
}

fn map_page_tables(
    cnode_cap: &mut CNodeCap,
    bootstage_mbr: &mut RootServerResourceManager,
    root_table_cap: &mut PageTableCap,
    vaddr_n: VirtAddr,
) {
    loop {
        let mut page_table_cap = PageTableCap::init(bootstage_mbr.alloc_page(), 0);
        cnode_cap
            .write_slot(page_table_cap.replicate(), bootstage_mbr.alloc_cnode_idx())
            .unwrap();
        if let Ok(level) = page_table_cap.map(root_table_cap, vaddr_n) {
            if level == 0 {
                break;
            }
        } else {
            panic!("error occur")
        }
    }
}

#[inline]
fn get_flags(flags: u32) -> usize {
    (if ProgramFlags::is_executable(flags) {
        PAGE_X
    } else {
        0
    } | if ProgramFlags::is_writable(flags) {
        PAGE_W
    } else {
        0
    } | if ProgramFlags::is_readable(flags) {
        PAGE_R
    } else {
        0
    })
}
