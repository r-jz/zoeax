use crate::address::KernelVAddress;
use crate::capability::page_table::PageCap;
use crate::capability::{cnode::CNodeCap, page_table::PageTableCap};
use crate::common::{ErrKind, IPCBuffer, KernelResult};
use crate::kerr;
use crate::list::ListItem;
use crate::object::PageTable;
use crate::println;

use crate::scheduler::push;
use core::ptr;
#[cfg(debug_assertions)]
use core::sync::atomic::{AtomicUsize, Ordering};

use super::cnode::CNodeEntry;
use super::page_table::Page;
use super::{CNode, CSlot, KObject};
pub use shared::registers::Register;
pub use shared::registers::Registers;
#[cfg(debug_assertions)]
static TCBIDX: AtomicUsize = AtomicUsize::new(0);

pub type ThreadControlBlock = ListItem<ThreadInfo>;

impl KObject for ListItem<ThreadInfo> {}

pub fn resume(thread: &mut ThreadControlBlock) {
    thread.resume();
    push(thread)
}

#[allow(dead_code)]
pub fn suspend(_thread: &mut ThreadControlBlock) {
    // TODO: Impl Double linked list
    // 1, check self status is Runnable.
    // 2, if true, then self.next.prev = self.prev and self.prev.next = self.next
    // (i.e take self out from runqueue)
    // then call self.suspend()
    todo!()
}

#[derive(PartialEq, Eq, Debug, Default)]
pub enum ThreadState {
    #[default]
    Inactive,
    Runnable,
    Blocked,
    Idle,
}

#[derive(Debug, Default)]
pub struct ThreadInfo {
    pub status: ThreadState,
    pub time_slice: usize,
    pub root_cnode: CSlot<CNode>,
    pub vspace: CSlot<PageTable>,
    pub registers: Registers,
    pub ipc_buffer: CSlot<Page>,
    pub badge: usize,
    #[cfg(debug_assertions)]
    pub tid: usize,
}

impl ThreadInfo {
    pub fn new() -> Self {
        let mut ret = Self::default();
        #[cfg(debug_assertions)]
        if cfg!(debug_assertions) {
            let tid = TCBIDX.fetch_add(1, Ordering::Relaxed) + 1;
            ret.tid = tid;
        }
        ret
    }
    pub fn resume(&mut self) {
        self.status = ThreadState::Runnable;
    }
    pub fn suspend(&mut self) {
        self.status = ThreadState::Blocked;
    }
    pub fn is_runnable(&self) -> bool {
        self.status == ThreadState::Runnable
    }

    pub fn set_ipc_msg(&mut self, ipc_buffer_ref: Option<&mut IPCBuffer>) {
        if let (Some(reciever_ref), Some(sender_ref)) = (self.ipc_buffer_ref(), ipc_buffer_ref) {
            unsafe { ptr::copy(sender_ref, reciever_ref, 1) }
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn ipc_buffer_ref(&self) -> Option<&mut IPCBuffer> {
        self.ipc_buffer.as_ref().map(|page_cap_e| {
            let page_cap = page_cap_e.cap_ref();
            let address: *mut IPCBuffer = KernelVAddress::from(page_cap.get_address()).into();
            unsafe { &mut *{ address } }
        })
    }
    pub fn set_timeout(&mut self, time_out: usize) {
        self.time_slice = time_out
    }

    pub const fn idle_init() -> Self {
        Self {
            status: ThreadState::Idle,
            time_slice: 0,
            root_cnode: None,
            vspace: None,
            registers: Registers::null(),
            ipc_buffer: None,
            badge: 0,
            #[cfg(debug_assertions)]
            tid: 0,
        }
    }

    pub unsafe fn activate_vspace(&mut self) {
        if let Err(e) = self.activate_vspace_inner() {
            println!("Error occured, {e:?}");
            PageTable::activate_kernel_table();
        }
    }

    unsafe fn activate_vspace_inner(&mut self) -> KernelResult<()> {
        let cap_entry = self
            .vspace
            .as_mut()
            .ok_or(kerr!(ErrKind::PageTableNotMappedYet))?;
        let pt_cap = cap_entry.cap_ref_mut();
        unsafe { pt_cap.activate() }
    }

    pub fn set_root_cspace(&mut self, cspace_cap: CNodeCap, parent: &mut CNodeEntry<CNode>) {
        // TODO: you should consider when already set.
        assert!(self.root_cnode.is_none(), "{:?}", self.root_cnode);
        let mut new_entry = CNodeEntry::new_with_rawcap(cspace_cap);
        new_entry.insert(parent.as_mut());
        self.root_cnode = Some(new_entry)
    }

    pub fn set_root_vspace(
        &mut self,
        vspace_cap: PageTableCap,
        parent: &mut CNodeEntry<PageTable>,
    ) {
        // TODO: you should consider when already set.
        assert!(self.vspace.is_none(), "{:?}", self.vspace);
        let mut new_entry = CNodeEntry::new_with_rawcap(vspace_cap);
        new_entry.insert(parent.as_mut());
        self.vspace = Some(new_entry)
    }

    pub fn set_ipc_buffer(&mut self, page_cap: PageCap, parent: &mut CNodeEntry<Page>) {
        // TODO: check right
        // TODO: you should consider when already set.
        assert!(self.ipc_buffer.is_none());
        let mut new_entry = CNodeEntry::new_with_rawcap(page_cap);
        new_entry.insert(parent);
        self.ipc_buffer = Some(new_entry)
    }
}
