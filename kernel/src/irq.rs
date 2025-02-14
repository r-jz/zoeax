use crate::{object::{CSlot, Notification}, riscv::r_sip, KernelResult};
use core::mem::MaybeUninit;

const MAX_IRQ: usize = 128; 

// TODO: Have to think why they are separanted.
static mut IRQ_STATS: [IRQStatus; MAX_IRQ + 1] = [IRQStatus::Inactive; MAX_IRQ + 1];
static mut IRQ_NODES: MaybeUninit<[CSlot<Notification>; MAX_IRQ + 1]> = MaybeUninit::uninit();

static mut CURRENT_IRQ: Option<usize> = None;

#[derive(Clone, Copy, Debug)]
pub enum IRQStatus {
    Inactive,
    IrqSignal,
    IrqTimer
}

pub unsafe fn handle_irq() {
    let activate = get_active_irq();
    if let Ok(irq_num) = activate {
        if let Some(irq_status) = IRQ_STATS.get(irq_num) {
            match irq_status {
                IRQStatus::Inactive => {
                    // mask irq
                },
                IRQStatus::IrqTimer => {
                    // tick_timer()
                    // reset_timer()
                },
                IRQStatus::IrqSignal => {
                    let not_cap = IRQ_NODES.assume_init_mut().get_mut(irq_num).unwrap().as_mut().unwrap();
                    // get notification cap
                    // send notification
                }
            }
        // exceed max irq
        } else {
            // mask irq
        }
        // ack irq
    } else {
        // mask irq
    }
    // 1, get irq number from plic
    // 2, if irq is to be driven to user process, wake it up
    // 3, mask irq
    // 2, if irq number is enabled, tell user process to wake up
    // 3, else, 
}

pub unsafe fn init_irq_nodes() {
    let ptr = IRQ_NODES.as_mut_ptr().cast::<CSlot<Notification>>();
    for pos in 0..MAX_IRQ + 1 {
        *ptr.add(pos) = None
    }
}

pub fn get_active_irq() -> KernelResult<usize> {
    unsafe {
        if let Some(current) = CURRENT_IRQ {
            Ok(current)
        } else {
            todo!();
            let sip_val = r_sip();
            let irq = plic_get_irq();
            // 2, get irq number from plic
            // -- From seL4 --
            /* QEMU bug requires external interrupts to be immediately claimed. For
             * other platforms, the claim is done in invokeIRQHandler_AckIRQ.
             */
            plic_complete_claim(irq);
        }
    }
}

pub fn ack_irq(irq_number: usize) {
    // check irq number is collect
    unsafe {
        CURRENT_IRQ = None
    }
}

fn plic_get_irq() -> usize {todo!()}
fn plic_complete_claim(irq: usize) {}
fn plic_mask_irq(irq: usize) {}
