use crate::list::{LinkedList, ListItem};
use crate::object::{Registers, ThreadControlBlock, ThreadInfo};
use crate::println;
use crate::riscv::{r_sstatus, w_sstatus, wfi, SSTATUS_SIE, SSTATUS_SPIE, SSTATUS_SPP};
use core::ptr;

// TODO: use once_cell
pub static mut IDLE_THREAD: ThreadControlBlock = ThreadControlBlock::new(ThreadInfo::idle_init());

// TODO: use unsafe_cell
pub static mut CURRENT_PROC: *mut ThreadControlBlock = ptr::null_mut();
pub const TICK_HZ: usize = 1000;
pub const TASK_QUANTUM: usize = 20 * (TICK_HZ / 1000); // 20 ms;

pub static mut CPU_VAR: CpuVar = CpuVar {
    sptop: 0,
    sscratch: 0,
    cur_reg_base: ptr::null_mut(),
};

// TODO: use unsafe_cell
static mut SCHEDULER: Scheduler = Scheduler::new();

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

// TODO: remove this attribute
#[allow(static_mut_refs)]
pub unsafe fn schedule() {
    if !SCHEDULER.requested {
        return;
    }
    let next = if let Some(next) = SCHEDULER.sched() {
        next.set_timeout(TASK_QUANTUM);
        if (*CURRENT_PROC).is_runnable() {
            SCHEDULER.push(CURRENT_PROC.as_mut().unwrap());
        }
        next
    } else {
        if (*CURRENT_PROC).is_runnable() {
            (*CURRENT_PROC).set_timeout(TASK_QUANTUM);
            return;
        }
        &raw mut IDLE_THREAD
    };
    // change page table
    (*next).activate_vspace();
    unsafe {
        CPU_VAR.cur_reg_base = &raw mut (&mut *next).registers;
    }
    CURRENT_PROC = next;
    SCHEDULER.requested = false;
}

pub fn create_idle_thread(stack_top: usize) {
    unsafe {
        IDLE_THREAD.registers.sepc = idle as *const () as usize;
        IDLE_THREAD.registers.sstatus = SSTATUS_SPP | SSTATUS_SPIE;
        IDLE_THREAD.registers.sp = stack_top;
        CURRENT_PROC = &raw mut IDLE_THREAD;
        CPU_VAR.cur_reg_base = &raw mut IDLE_THREAD.registers;
        CPU_VAR.sptop = stack_top;
    }
}

#[no_mangle]
fn idle() -> ! {
    println!("In the Idle");
    loop {
        w_sstatus(r_sstatus() | SSTATUS_SIE);
        wfi();
    }
}

// TODO: remove this attribute
#[allow(static_mut_refs)]
pub fn push(tcb: &mut ThreadControlBlock) {
    unsafe { SCHEDULER.push(tcb) }
}

pub fn get_current_tcb_mut<'a>() -> &'a mut ThreadControlBlock {
    unsafe { &mut *CURRENT_PROC }
}

#[allow(static_mut_refs)]
pub fn require_schedule() {
    unsafe { SCHEDULER.requested = true }
}

pub fn timer_tick() {
    unsafe {
        if CURRENT_PROC == &raw mut IDLE_THREAD {
            return;
        }
        (&mut *CURRENT_PROC).time_slice -= 1;
        if (&*CURRENT_PROC).time_slice == 0 {
            require_schedule()
        }
    }
}

pub fn get_current_reg<'a>() -> &'a mut Registers {
    unsafe { &mut *(CPU_VAR.cur_reg_base) }
}
