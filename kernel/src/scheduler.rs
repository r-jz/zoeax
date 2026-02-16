use crate::common::SyncUnsafeCell;
use crate::list::{LinkedList, ListItem};
use crate::object::{Registers, ThreadControlBlock, ThreadInfo};
use crate::println;
use crate::riscv::{r_sstatus, w_sstatus, wfi, SSTATUS_SIE, SSTATUS_SPIE, SSTATUS_SPP};
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

pub const TICK_HZ: usize = 1000;
pub const TASK_QUANTUM: usize = 20 * (TICK_HZ / 1000); // 20 ms;

static IDLE_THREAD: SyncUnsafeCell<ThreadControlBlock> =
    SyncUnsafeCell::new(ThreadControlBlock::new(ThreadInfo::idle_init()));
static CURRENT_PROC: AtomicPtr<ThreadControlBlock> = AtomicPtr::new(ptr::null_mut());
static CPU_VAR: SyncUnsafeCell<CpuVar> = SyncUnsafeCell::new(CpuVar {
    sptop: 0,
    sscratch: 0,
    cur_reg_base: ptr::null_mut(),
});
static SCHEDULER: SyncUnsafeCell<Scheduler> = SyncUnsafeCell::new(Scheduler::new());

#[repr(C)]
#[derive(Debug)]
pub struct CpuVar {
    pub sptop: usize,
    pub sscratch: usize,
    pub cur_reg_base: *mut Registers,
}

#[derive(Default)]
pub struct Scheduler {
    runqueue: LinkedList<ThreadInfo>,
    requested: bool,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            runqueue: LinkedList::new(),
            requested: false,
        }
    }

    pub fn push(&mut self, proc: &mut ThreadControlBlock) {
        self.runqueue.push(proc);
    }

    pub fn sched(&mut self) -> Option<&mut ListItem<ThreadInfo>> {
        self.runqueue.pop()
    }
}

pub unsafe fn schedule() {
    let scheduler = unsafe { &mut *SCHEDULER.get() };
    if !scheduler.requested {
        return;
    }
    let next = if let Some(next) = scheduler
        .sched()
        .map(|next| next as *mut ThreadControlBlock)
    {
        let next = unsafe { &mut *next };
        next.set_timeout(TASK_QUANTUM);
        let current = CURRENT_PROC.load(Ordering::Relaxed);
        if unsafe { (*current).is_runnable() } {
            scheduler.push(unsafe { current.as_mut().unwrap() });
        }
        next as *mut ThreadControlBlock
    } else {
        let current = CURRENT_PROC.load(Ordering::Relaxed);
        if unsafe { (*current).is_runnable() } {
            unsafe { (*current).set_timeout(TASK_QUANTUM) };
            return;
        }
        IDLE_THREAD.get()
    };
    // change page table
    unsafe { (*next).activate_vspace() };
    let cpu_var = unsafe { &mut *CPU_VAR.get() };
    cpu_var.cur_reg_base = &raw mut (&mut *next).registers;
    CURRENT_PROC.store(next, Ordering::Relaxed);
    scheduler.requested = false;
}

pub fn create_idle_thread(stack_top: usize) {
    let idle_tcb = unsafe { &mut *IDLE_THREAD.get() };
    idle_tcb.registers.sepc = idle as *const () as usize;
    idle_tcb.registers.sstatus = SSTATUS_SPP | SSTATUS_SPIE;
    idle_tcb.registers.sp = stack_top;
    CURRENT_PROC.store(idle_tcb as *mut ThreadControlBlock, Ordering::Relaxed);

    let cpu_var = unsafe { &mut *CPU_VAR.get() };
    cpu_var.cur_reg_base = &raw mut idle_tcb.registers;
    cpu_var.sptop = stack_top;
}

#[no_mangle]
fn idle() -> ! {
    println!("In the Idle");
    loop {
        w_sstatus(r_sstatus() | SSTATUS_SIE);
        wfi();
    }
}

pub fn push(tcb: &mut ThreadControlBlock) {
    unsafe { (&mut *SCHEDULER.get()).push(tcb) }
}

pub fn get_current_tcb_mut<'a>() -> &'a mut ThreadControlBlock {
    let current = CURRENT_PROC.load(Ordering::Relaxed);
    unsafe { &mut *current }
}

pub fn require_schedule() {
    unsafe { (&mut *SCHEDULER.get()).requested = true }
}

pub fn timer_tick() {
    let current = CURRENT_PROC.load(Ordering::Relaxed);
    if ptr::eq(current, IDLE_THREAD.get()) {
        return;
    }
    unsafe {
        (&mut *current).time_slice -= 1;
        if (&*current).time_slice == 0 {
            require_schedule()
        }
    }
}

pub fn get_current_reg<'a>() -> &'a mut Registers {
    let cpu_var = unsafe { &mut *CPU_VAR.get() };
    unsafe { &mut *(cpu_var.cur_reg_base) }
}

pub fn cpu_var_ptr() -> usize {
    CPU_VAR.get() as usize
}
