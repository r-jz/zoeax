#![allow(dead_code, static_mut_refs)]

use shared::err_kind::ErrKind;

use crate::{
    address::PhysAddr,
    kerr,
    memlayout::PLIC,
    object::{CNodeEntry, CSlot, Notification},
    println,
    riscv::{r_sip, SIP_STIP},
    KernelResult,
};
use core::mem::MaybeUninit;

const MAX_IRQ: usize = 128;

// TODO: Have to think why they are separanted.
static mut IRQ_STATS: [IRQStatus; MAX_IRQ] = [IRQStatus::Inactive; MAX_IRQ];
static mut IRQ_NODES: MaybeUninit<[CSlot<Notification>; MAX_IRQ]> = MaybeUninit::uninit();

static mut CURRENT_IRQ: Option<IRQReason> = None;

#[derive(Clone, Copy, Debug)]
enum IRQReason {
    Timer,
    External(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IRQStatus {
    Inactive,
    IrqSignal,
}

pub fn handle_irq_entry() {
    if let Ok(irq_reason) = get_active_irq() {
        match irq_reason {
            IRQReason::Timer => ack_irq(0),
            IRQReason::External(irq_num) => unsafe {
                handle_irq(irq_num as usize);
            },
        }
    }
}

unsafe fn handle_irq(irq_num: usize) {
    match IRQ_STATS.get(irq_num) {
        // exceed max irq number
        None => mask_interrupt(true, irq_num),
        Some(IRQStatus::Inactive) => mask_interrupt(true, irq_num),
        Some(IRQStatus::IrqSignal) => {
            if let Some(not_cap) = IRQ_NODES
                .assume_init_mut()
                .get_mut(irq_num)
                .unwrap()
                .as_mut()
            {
                not_cap.cap_ref_mut().send();
            } else {
                println!("no notification cap is set.");
            }
            mask_interrupt(true, irq_num);
        }
    };
    // exceed max irq
    ack_irq(irq_num);
}

pub fn activate_irq(irq_number: usize) -> KernelResult<()> {
    let irq_status = unsafe {
        IRQ_STATS
            .get_mut(irq_number)
            .ok_or(kerr!(ErrKind::UnknownIRQ))?
    };
    (!(irq_status == &mut IRQStatus::IrqSignal))
        .then_some(())
        .ok_or(kerr!(ErrKind::IRQAlreadyActive))?;
    mask_interrupt(false, irq_number);
    *irq_status = IRQStatus::IrqSignal;
    Ok(())
}

pub fn set_irq(irq_number: usize, not_slot: &mut CNodeEntry<Notification>) {
    let irq_hander = unsafe { IRQ_NODES.assume_init_mut().get_mut(irq_number).unwrap() };
    let mut new_slot = CNodeEntry::new_with_rawcap(not_slot.cap_ref().replicate());
    new_slot.insert(not_slot);
    *irq_hander = Some(new_slot)
}

pub fn mask_interrupt(disable: bool, irq_number: usize) {
    plic_mask_irq(disable, irq_number)
}

pub unsafe fn init_irq_nodes() {
    // call only once
    let ptr = IRQ_NODES.as_mut_ptr().cast::<CSlot<Notification>>();
    for pos in 0..MAX_IRQ {
        *ptr.add(pos) = None
    }
}

fn get_active_irq() -> KernelResult<IRQReason> {
    unsafe {
        if let Some(current) = CURRENT_IRQ {
            Ok(current)
        } else {
            let sip_val = r_sip();
            let irq = plic_get_irq();
            let ret = {
                if is_external(sip_val) {
                    let irq_num = plic_get_irq();
                    // -- From seL4 --
                    /* QEMU bug requires external interrupts to be immediately claimed. For
                     * other platforms, the claim is done in invokeIRQHandler_AckIRQ.
                     */
                    plic_complete_claim(irq);
                    Ok(IRQReason::External(irq_num as u8))
                } else if is_timer(sip_val) {
                    Ok(IRQReason::Timer)
                } else {
                    /* Seems none of the known sources has a pending interrupt. This can
                     * happen if e.g. if another hart context has claimed the interrupt
                     * already.
                     */
                    Err(kerr!(ErrKind::IRQInvalid))
                }
            };
            if let Ok(reason) = ret {
                CURRENT_IRQ = Some(reason)
            }
            ret
        }
    }
}

#[inline]
fn is_external(sip_val: usize) -> bool {
    const SIP_SEIP: usize = 1 << 9;
    (sip_val & SIP_SEIP) != 0
}

fn is_timer(sip_val: usize) -> bool {
    (sip_val & SIP_STIP) != 0
}

fn ack_irq(_irq_number: usize) {
    // check irq number is collect
    unsafe { CURRENT_IRQ = None }
}

fn plic_get_irq() -> usize {
    unsafe { plic_sclaim_ptr().read_volatile() as usize }
}
fn plic_complete_claim(irq: usize) {
    unsafe {
        plic_sclaim_ptr().write_volatile(irq as u32);
    }
}
fn plic_mask_irq(disable: bool, irq: usize) {
    if irq == 0 {
        return;
    }
    let bit = 1u32 << (irq % 32);
    unsafe {
        let enable_ptr = plic_senable_ptr(irq / 32);
        let mut val = enable_ptr.read_volatile();
        if disable {
            val &= !bit;
        } else {
            val |= bit;
        }
        enable_ptr.write_volatile(val);
    }
}

fn plic_senable_ptr(word_idx: usize) -> *mut u32 {
    // S-mode enable base: PLIC + 0x2080 (+ hart * 0x100), hart=0.
    let paddr = PhysAddr::from(PLIC + 0x2080 + word_idx * core::mem::size_of::<u32>());
    let vaddr: *mut u32 = paddr.into();
    vaddr
}

fn plic_sclaim_ptr() -> *mut u32 {
    // S-mode claim/complete: PLIC + 0x201004 (+ hart * 0x2000), hart=0.
    let paddr = PhysAddr::from(PLIC + 0x201004);
    let vaddr: *mut u32 = paddr.into();
    vaddr
}
