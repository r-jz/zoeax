use core::{
    arch::{asm, naked_asm},
    mem::offset_of,
};

use crate::{
    object::Registers,
    riscv::{r_scause, r_sepc, r_stval},
    scheduler::{get_current_reg, get_current_tcb_mut, schedule, timer_tick, CpuVar},
    syscall::handle_syscall,
    timer::set_timer,
};

// I wanna use enum;
/// Interrupts,
const SUPERVISORSOFTWARE: usize = 1;
const SUPREVISORTIMER: usize = 5;
const SUPREVISOREXTERNAL: usize = 9;
const COUNTER_OVERFLOW: usize = 13;

/// Exceptions
const IMISSALIGNED: usize = 0;
const IACCESSFAULT: usize = 1;
const ILEAGALI: usize = 2;
const BREAKPOINT: usize = 3;
const LMISSALIGNED: usize = 4;
const LACCESSFAULT: usize = 5;
const SAMISSALIGNED: usize = 6;
const SAACCESSFAULT: usize = 7;
const ECALLUSER: usize = 8;
const ECALLSUPERVIOSR: usize = 9;
const IPAGEFAULT: usize = 12;
const LPAGEFAULT: usize = 13;
const SAPAGEFAULT: usize = 15;

/// This is trap handler entry fnction.
/// Save context when trap was occured (trap frame),
/// then call trap_handler
/// after that, restore context and call sret.
#[unsafe(naked)]
#[allow(dead_code)]
pub extern "C" fn trap_entry() {
    unsafe {
        naked_asm!(
        ".balign 8",

        // sscratch has cpu var address
        // tmp = tp
        // tp = &CPU_VAR
        // sscratch = tmp
        "csrrw tp, sscratch, tp",

        // CPU_VAR.sscratch = a0
        "sd t0, {sscratch_offset}(tp)",

        // load current thread register base to a0
        "ld t0, {cur_reg_base_offset}(tp)",

        "sd ra, {ra_offset}(t0)",
        "sd sp, {sp_offset}(t0)",
        "sd gp, {gp_offset}(t0)",
        // t0 and tp was skipped because they will be restored after.
        "sd t1, {t1_offset}(t0)",
        "sd t2, {t2_offset}(t0)",
        "sd s0, {s0_offset}(t0)",
        "sd s1, {s1_offset}(t0)",
        "sd a0, {a0_offset}(t0)",
        "sd a1, {a1_offset}(t0)",
        "sd a2, {a2_offset}(t0)",
        "sd a3, {a3_offset}(t0)",
        "sd a4, {a4_offset}(t0)",
        "sd a5, {a5_offset}(t0)",
        "sd a6, {a6_offset}(t0)",
        "sd a7, {a7_offset}(t0)",
        "sd s2, {s2_offset}(t0)",
        "sd s3, {s3_offset}(t0)",
        "sd s4, {s4_offset}(t0)",
        "sd s5, {s5_offset}(t0)",
        "sd s6, {s6_offset}(t0)",
        "sd s7, {s7_offset}(t0)",
        "sd s8, {s8_offset}(t0)",
        "sd s9, {s9_offset}(t0)",
        "sd s10, {s10_offset}(t0)",
        "sd s11, {s11_offset}(t0)",
        "sd t3, {t3_offset}(t0)",
        "sd t4, {t4_offset}(t0)",
        "sd t5, {t5_offset}(t0)",
        "sd t6, {t6_offset}(t0)",

        "csrr a1, sepc",
        "sd a1, {sepc_offset}(t0)",
        "csrr a1, sstatus",
        "sd a1, {sstatus_offset}(t0)",

        "csrrw a1, sscratch, tp",
        "sd a1, {tp_offset}(t0)",

        // restore t0
        "ld a1, {sscratch_offset}(tp)",
        "sd a1, {t0_offset}(t0)",

        "ld sp, {sptop_offset}(tp)",
        "csrr a1, scause",
        "sd a1, {scause_offset}(t0)",

        "j {handle_trap}",

        handle_trap = sym handle_trap,
        sscratch_offset = const offset_of!(CpuVar, sscratch),
        cur_reg_base_offset = const offset_of!(CpuVar, cur_reg_base),
        sptop_offset = const offset_of!(CpuVar, sptop),
        ra_offset = const offset_of!(Registers, ra),
        sp_offset = const offset_of!(Registers, sp),
        gp_offset = const offset_of!(Registers, gp),
        tp_offset = const offset_of!(Registers, tp),
        t0_offset = const offset_of!(Registers, t0),
        t1_offset = const offset_of!(Registers, t1),
        t2_offset = const offset_of!(Registers, t2),
        s0_offset = const offset_of!(Registers, s0),
        s1_offset = const offset_of!(Registers, s1),
        a0_offset = const offset_of!(Registers, a0),
        a1_offset = const offset_of!(Registers, a1),
        a2_offset = const offset_of!(Registers, a2),
        a3_offset = const offset_of!(Registers, a3),
        a4_offset = const offset_of!(Registers, a4),
        a5_offset = const offset_of!(Registers, a5),
        a6_offset = const offset_of!(Registers, a6),
        a7_offset = const offset_of!(Registers, a7),
        s2_offset = const offset_of!(Registers, s2),
        s3_offset = const offset_of!(Registers, s3),
        s4_offset = const offset_of!(Registers, s4),
        s5_offset = const offset_of!(Registers, s5),
        s6_offset = const offset_of!(Registers, s6),
        s7_offset = const offset_of!(Registers, s7),
        s8_offset = const offset_of!(Registers, s8),
        s9_offset = const offset_of!(Registers, s9),
        s10_offset = const offset_of!(Registers, s10),
        s11_offset = const offset_of!(Registers, s11),
        t3_offset = const offset_of!(Registers, t3),
        t4_offset = const offset_of!(Registers, t4),
        t5_offset = const offset_of!(Registers, t5),
        t6_offset = const offset_of!(Registers, t6),
        scause_offset = const offset_of!(Registers, scause),
        sstatus_offset = const offset_of!(Registers, sstatus),
        sepc_offset = const offset_of!(Registers, sepc),
        )
    }
}

#[no_mangle]
fn handle_trap() -> ! {
    let scause = r_scause();
    let code = scause & !(1 << (usize::BITS - 1));
    let stval = r_stval();
    let user_pc = r_sepc();

    if (scause >> (usize::BITS - 1)) == 1 {
        //  interrupt
        match code {
            SUPREVISORTIMER => {
                timer_tick();
                set_timer(10000);
            }
            SUPERVISORSOFTWARE => {
                panic!(
                    "supervisor software scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                )
            }
            SUPREVISOREXTERNAL => {
                panic!(
                    "supervisor external scause={:x}, stval={:x}, sepc={:x}",
                    code, stval, user_pc
                )
            }
            COUNTER_OVERFLOW => {
                panic!(
                    "counter overflow scause={:x}, stval={:x}, sepc={:x}",
                    code, stval, user_pc
                )
            }
            _ => {
                panic!(
                    "unexpected interrupt scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
        }
    } else {
        match code {
            ECALLUSER => {
                let reg = get_current_reg();
                let syscall_n = reg.a7;
                handle_syscall(syscall_n, reg);
            }
            IMISSALIGNED => {
                panic!(
                    "inst missaligned scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            IACCESSFAULT => {
                panic!(
                    "inst access fault scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            ILEAGALI => {
                panic!(
                    "inst ileagal scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            BREAKPOINT => {
                panic!(
                    "break point scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            LMISSALIGNED => {
                panic!(
                    "load missaligned scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            LACCESSFAULT => {
                panic!(
                    "load access fault scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            SAMISSALIGNED => {
                panic!(
                    "store/amo missaligned scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            SAACCESSFAULT => {
                panic!(
                    "store/amo access fault scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            ECALLSUPERVIOSR => {
                panic!(
                    "ecall from supervisor scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            IPAGEFAULT => {
                panic!(
                    "inst page fault scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            LPAGEFAULT => {
                panic!(
                    "load page fault scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            SAPAGEFAULT => {
                panic!(
                    "store/amo page fault, scause={:x}, stval={:x}, sepc={:x}",
                    scause, stval, user_pc
                );
            }
            _ => {
                panic!(
                    "unexpected exception scause={:x}, stval={:x}, sepc={:x}, code={:x}",
                    scause, stval, user_pc, code
                );
            }
        }
    }
    unsafe {
        schedule();
        return_to_user()
    }
}

#[no_mangle]
pub unsafe fn return_to_user() -> ! {
    let tcb = get_current_tcb_mut();
    let address = &raw mut tcb.registers;

    asm!(
        // restore registers
        // t0 is used to store current tcb's registers base address,
        // t1 will be used to restore sregs.

        "ld ra, {ra_offset}(t0)",
        "ld sp, {sp_offset}(t0)",
        "ld gp, {gp_offset}(t0)",
        "ld tp, {tp_offset}(t0)",
        // t0 and t1 was skipped because they will be restored after.
        "ld t2, {t2_offset}(t0)",
        "ld s0, {s0_offset}(t0)",
        "ld s1, {s1_offset}(t0)",
        "ld a0, {a0_offset}(t0)",
        "ld a1, {a1_offset}(t0)",
        "ld a2, {a2_offset}(t0)",
        "ld a3, {a3_offset}(t0)",
        "ld a4, {a4_offset}(t0)",
        "ld a5, {a5_offset}(t0)",
        "ld a6, {a6_offset}(t0)",
        "ld a7, {a7_offset}(t0)",

        "ld s2, {s2_offset}(t0)",
        "ld s3, {s3_offset}(t0)",
        "ld s4, {s4_offset}(t0)",
        "ld s5, {s5_offset}(t0)",
        "ld s6, {s6_offset}(t0)",
        "ld s7, {s7_offset}(t0)",
        "ld s8, {s8_offset}(t0)",
        "ld s9, {s9_offset}(t0)",
        "ld s10, {s10_offset}(t0)",
        "ld s11, {s11_offset}(t0)",
        "ld t3, {t3_offset}(t0)",
        "ld t4, {t4_offset}(t0)",
        "ld t5, {t5_offset}(t0)",
        "ld t6, {t6_offset}(t0)",

        // restore sepc
        "ld t1, {sepc_offset}(t0)",
        "csrw sepc, t1",

        // restore sstatus
        "ld t1, {sstatus_offset}(t0)",
        "csrw sstatus, t1",

        // restore t1 and t0
        "ld t1, {t1_offset}(t0)",
        "ld t0, {t0_offset}(t0)",

        "sret",
        in ("t0") address as usize,
        ra_offset = const offset_of!(Registers, ra),
        sp_offset = const offset_of!(Registers, sp),
        gp_offset = const offset_of!(Registers, gp),
        tp_offset = const offset_of!(Registers, tp),
        t0_offset = const offset_of!(Registers, t0),
        t1_offset = const offset_of!(Registers, t1),
        t2_offset = const offset_of!(Registers, t2),
        s0_offset = const offset_of!(Registers, s0),
        s1_offset = const offset_of!(Registers, s1),
        a0_offset = const offset_of!(Registers, a0),
        a1_offset = const offset_of!(Registers, a1),
        a2_offset = const offset_of!(Registers, a2),
        a3_offset = const offset_of!(Registers, a3),
        a4_offset = const offset_of!(Registers, a4),
        a5_offset = const offset_of!(Registers, a5),
        a6_offset = const offset_of!(Registers, a6),
        a7_offset = const offset_of!(Registers, a7),
        s2_offset = const offset_of!(Registers, s2),
        s3_offset = const offset_of!(Registers, s3),
        s4_offset = const offset_of!(Registers, s4),
        s5_offset = const offset_of!(Registers, s5),
        s6_offset = const offset_of!(Registers, s6),
        s7_offset = const offset_of!(Registers, s7),
        s8_offset = const offset_of!(Registers, s8),
        s9_offset = const offset_of!(Registers, s9),
        s10_offset = const offset_of!(Registers, s10),
        s11_offset = const offset_of!(Registers, s11),
        t3_offset = const offset_of!(Registers, t3),
        t4_offset = const offset_of!(Registers, t4),
        t5_offset = const offset_of!(Registers, t5),
        t6_offset = const offset_of!(Registers, t6),
        sstatus_offset = const offset_of!(Registers, sstatus),
        sepc_offset = const offset_of!(Registers, sepc),
        options(noreturn)
    )
}
