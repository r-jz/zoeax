use self::root_server::{
    ROOT_BOOT_INFO_PAGE, ROOT_CNODE_ENTRY_NUM_BITS, ROOT_CNODE_IDX, ROOT_VSPACE_IDX,
};
use crate::address::PAGE_SIZE;
use crate::common::{BootInfo, UntypedInfo};
use crate::scheduler::{create_idle_thread, require_schedule, schedule};
use shared::elf::def::Elf64Hdr;
use shared::registers::Register;

mod pm;
mod root_server;
mod vm;

use crate::println;
use crate::riscv::{r_sie, w_sie, w_sscratch, w_stvec, SIE_SEIE, SIE_SSIE, SIE_STIE};
use crate::scheduler::cpu_var_ptr;
use crate::timer::{set_timer, MTIME_PER_1MS};
use crate::trap::trap_entry;
use pm::BumpAllocator;
use root_server::{set_device_memory, RootServerMemory, RootServerResourceManager};
use vm::kernel_vm_init;

extern "C" {
    static __stack_top: u8;
}

pub fn init_kernel(elf_header: *const Elf64Hdr, free_ram_phys: usize, free_ram_end_phys: usize) {
    println!("initialising kernel");
    w_stvec(trap_entry as *const () as usize);
    let bump_allocator = unsafe { BumpAllocator::new(free_ram_phys, free_ram_end_phys) };
    unsafe { kernel_vm_init(free_ram_end_phys) };
    w_sie(r_sie() | SIE_SEIE | SIE_STIE | SIE_SSIE);
    init_root_server(bump_allocator, elf_header);
    w_sscratch(cpu_var_ptr());
    set_timer(MTIME_PER_1MS);
    println!("initialization finished");
}

fn create_initial_thread(
    root_server_mem: &mut RootServerMemory,
    mut bootrsc_mgr: RootServerResourceManager,
    elf_header: *const Elf64Hdr,
) {
    let mut root_cnode_cap = root_server_mem.create_root_cnode();

    let (mut vspace_cap, max_vaddr) =
        root_server_mem.create_address_space(&mut root_cnode_cap, elf_header, &mut bootrsc_mgr);

    root_server_mem.create_irqs(&mut root_cnode_cap);

    let mut ipc_page_cap = root_server_mem.create_ipc_buf_frame(
        &mut root_cnode_cap,
        &mut vspace_cap,
        max_vaddr,
        &mut bootrsc_mgr,
    );

    let (_, boot_info_addr) = root_server_mem.create_boot_info_frame(
        &mut root_cnode_cap,
        &mut vspace_cap,
        max_vaddr.add(PAGE_SIZE),
        &mut bootrsc_mgr,
    );

    create_idle_thread(&raw const __stack_top as usize);
    let boot_info_ptr: *mut BootInfo = boot_info_addr.into();
    let boot_info = unsafe {
        *boot_info_ptr = BootInfo::default();
        boot_info_ptr.as_mut().unwrap()
    };

    let entry_point = unsafe { (*elf_header).e_entry };
    let mut root_tcb = root_server_mem.create_root_tcb(
        &mut root_cnode_cap,
        &mut vspace_cap,
        &mut ipc_page_cap,
        entry_point.into(),
    );

    // 6, convert rest of memory into untyped objects.
    let mut num = 0;
    let mut max_idx = 0;
    for (idx, (untyped_cap_idx, untyped_cap)) in bootrsc_mgr.finalize().enumerate() {
        assert!(num < 32);
        root_cnode_cap
            .write_slot(untyped_cap.replicate(), untyped_cap_idx)
            .unwrap();
        boot_info.untyped_infos[idx] = UntypedInfo {
            bits: untyped_cap.block_size(),
            idx: untyped_cap_idx,
            is_device: false,
            phys_addr: untyped_cap.get_address().into(),
        };
        num += 1;
        boot_info.firtst_empty_idx = untyped_cap_idx + 1;
        max_idx = idx;
    }

    // dirty hacking function
    boot_info.untyped_num = num;
    for (i, ch) in "hello, root_server\n".as_bytes().iter().enumerate() {
        boot_info.msg[i] = *ch;
    }
    set_device_memory(&mut root_cnode_cap, boot_info, max_idx + 1);
    boot_info.root_cnode_idx = ROOT_CNODE_IDX;
    boot_info.root_vspace_idx = ROOT_VSPACE_IDX;
    boot_info.ipc_buffer_addr = max_vaddr.add(PAGE_SIZE).into();
    // 7, set initial thread into current thread
    root_tcb.set_register(&[(Register::A0, max_vaddr.add(PAGE_SIZE * 2).into())]);
    root_tcb.make_runnable();
    println!("root process initialization finished");
}

pub fn init_root_server(mut bump_allocator: BumpAllocator, elf_header: *const Elf64Hdr) {
    let mut root_server_mem = RootServerMemory::init_with_uninit(&mut bump_allocator);
    let bootstage_mbr = RootServerResourceManager::new(
        bump_allocator,
        ROOT_BOOT_INFO_PAGE + 1,
        2_usize.pow(ROOT_CNODE_ENTRY_NUM_BITS as u32) - 1,
    );
    create_initial_thread(&mut root_server_mem, bootstage_mbr, elf_header);

    require_schedule();
    unsafe {
        schedule();
    }
}
