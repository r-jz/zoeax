use crate::object::{CSlot, Notification};
use core::mem::MaybeUninit;

const MAX_IRQ: usize = 128; 

static mut IRQ_STATS: [IRQStatus; MAX_IRQ + 1] = [IRQStatus::Inactive; MAX_IRQ + 1];
static mut IRQ_NODES: MaybeUninit<[CSlot<Notification>; MAX_IRQ + 1]> = MaybeUninit::uninit();

#[derive(Clone, Copy, Debug)]
pub enum IRQStatus {
    Inactive,
    IrqSignal,
    IrqTimer
}


pub fn handle_irq(irq_number: usize) -> () {}

pub unsafe fn init_irq_nodes() {
    let ptr = IRQ_NODES.as_mut_ptr().cast::<CSlot<Notification>>();
    for pos in 0..MAX_IRQ + 1 {
        *ptr.add(pos) = None
    }
}
