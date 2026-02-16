/* ------------------------------------------------------------
 * Constants
 * ------------------------------------------------------------ */
.equ PAGE_SHIFT,   12
.equ PAGE_SIZE,    (1 << PAGE_SHIFT)

.equ PTE_VALID,    (1 << 0)
.equ PTE_READ,     (1 << 1)
.equ PTE_WRITE,    (1 << 2)
.equ PTE_EXECUTE,  (1 << 3)

/* satp.MODE for RV64 */
.equ SATP_MODE_SV48, 9
.equ SATP_MODE_SHIFT, 60

/* ------------------------------------------------------------
 * Boot text
 * ------------------------------------------------------------ */
.section ".boot.text", "ax"
.global boot

/* ------------------------------------------------------------
 * Helper: Jump absolute via nearby literal (robust against relax)
 * ------------------------------------------------------------ */
.macro JUMP_ABS, target
    .option push
    .option norelax
    j      2f
    .align 3
1:  .dword \target
2:
3:  auipc  t0, %pcrel_hi(1b)
    ld     t0, %pcrel_lo(3b)(t0)
    jr     t0
    .option pop
.endm

/* ------------------------------------------------------------
 * Helper: Load absolute address of a (possibly far) symbol into reg
 * Uses literal pool + PC-relative load
 * ------------------------------------------------------------ */
.macro LA_FAR, reg, sym
    .option push
    .option norelax
    j      2f
    .align 3
1:  .dword \sym
2:
3:  auipc  \reg, %pcrel_hi(1b)
    ld     \reg, %pcrel_lo(3b)(\reg)
    .option pop
.endm
/* ------------------------------------------------------------
 * PTE helpers
 * ------------------------------------------------------------ */

/* reg = (addr(sym) >> PAGE_SHIFT)  => PPN */
.macro PPN, reg, sym
    la     \reg, \sym
    srli   \reg, \reg, PAGE_SHIFT
.endm

/* Extract VPN[lvl] from VA in t1, return byte offset in t1 (index*8) */
.macro VPN_OFFSET_FROM_T1, lvl
    srli   t1, t1, (12 + 9 * \lvl)
    andi   t1, t1, 0x1ff
    slli   t1, t1, 3        /* *8 bytes per PTE */
.endm

/* Write PTE: pt is symbol base, offset in t1, ppn in ppn_reg, flags imm */
.macro WRITE_PTE, pt, ppn_reg, flags
    la     t0, \pt
    add    t0, t0, t1

    /* PTE = (PPN<<10) | flags */
    slli   \ppn_reg, \ppn_reg, 10
    addi   \ppn_reg, \ppn_reg, \flags

    sd     \ppn_reg, 0(t0)
.endm

/* Set PTE for VA symbol (near) */
.macro PTE_SET, pt, va_sym, lvl, ppn_reg, flags
    la     t1, \va_sym
    VPN_OFFSET_FROM_T1 \lvl
    WRITE_PTE \pt, \ppn_reg, \flags
.endm

/* Set PTE for VA symbol (far) */
.macro PTE_SET_FAR, pt, va_sym, lvl, ppn_reg, flags
    LA_FAR t1, \va_sym
    VPN_OFFSET_FROM_T1 \lvl
    WRITE_PTE \pt, \ppn_reg, \flags
.endm

/* ------------------------------------------------------------
 * Entry
 * ------------------------------------------------------------ */
boot:
    /* Make control flow explicit: boot -> _abs_start */
    JUMP_ABS _abs_start

_abs_start:
    .option norelax

    /* Disable S-mode interrupts early */
    csrw sie, zero
    csrw sip, zero

    /* --------------------------------------------------------
     * Page table setup (Sv48):
     *   - identity map: 1GB at _RAM_START (L2 leaf)
     *   - higher-half map: 1GB kernel VA -> _RAM_START (L2 leaf)
     * Root: __page_kernel_level_3 (L3)
     * -------------------------------------------------------- */

    /* L3: entry for identity region -> __page_init_id_lvl2 */
    PPN t2, __page_init_id_lvl2
    PTE_SET __page_kernel_level_3, _RAM_START, 3, t2, PTE_VALID

    /* L2: 1GB identity leaf mapping at _RAM_START */
    PPN t2, _RAM_START
    PTE_SET __page_init_id_lvl2, _RAM_START, 2, t2, (PTE_VALID | PTE_EXECUTE | PTE_READ | PTE_WRITE)

    /* L3: entry for higher-half kernel VA -> __page_kernel_lvl2 */
    PPN t2, __page_kernel_lvl2
    PTE_SET_FAR __page_kernel_level_3, _KERNEL_CODE_VIRTUAL, 3, t2, PTE_VALID

    /* L2: 1GB kernel mapping: VA(_KERNEL_CODE_VIRTUAL) -> PA(_RAM_START) */
    PPN t2, _RAM_START
    PTE_SET_FAR __page_kernel_lvl2, _KERNEL_CODE_VIRTUAL, 2, t2, (PTE_VALID | PTE_EXECUTE | PTE_READ | PTE_WRITE)

    /* --------------------------------------------------------
     * Enable paging: satp = MODE(Sv48) | root_ppn
     * -------------------------------------------------------- */
    li     t1, SATP_MODE_SV48
    slli   t1, t1, SATP_MODE_SHIFT
    PPN    t0, __page_kernel_level_3
    or     t0, t0, t1

    sfence.vma x0, x0
    csrw   satp, t0
    sfence.vma x0, x0

    /* --------------------------------------------------------
     * Clear registers (keep a0-a2)
     * -------------------------------------------------------- */
    li x1,  0
    /* x2 is sp, will be set later */
    li x3,  0
    li x4,  0
    li x5,  0
    li x6,  0
    li x7,  0
    li x8,  0
    li x9,  0
    /* skip x10,x11,x12 (a0,a1,a2) */
    li x13, 0
    li x14, 0
    li x15, 0
    li x16, 0
    li x17, 0
    li x18, 0
    li x19, 0
    li x20, 0
    li x21, 0
    li x22, 0
    li x23, 0
    li x24, 0
    li x25, 0
    li x26, 0
    li x27, 0
    li x28, 0
    li x29, 0
    li x30, 0
    li x31, 0

    /* --------------------------------------------------------
     * Pass free RAM range in a2/a3 (choose one method)
     *   - If these are far symbols, use LA_FAR for robustness.
     * -------------------------------------------------------- */
    LA_FAR a2, __free_ram_phys
    LA_FAR a3, __free_ram_end_phys

    /* setup global pointer (see linker script hades.x) */
    LA_FAR gp, __global_pointer$

    /* setup stack (virtual address) */
    LA_FAR sp, __stack_top

    /* jump to kernel main (virtual) */
    LA_FAR t0, _kernel_main
    jalr   zero, 0(t0)

    /* should not return */
1:  j 1b

/* ------------------------------------------------------------
 * Page table pages (4KiB each, zeroed)
 * ------------------------------------------------------------ */
.macro DEFINE_PAGE, name
    .align PAGE_SHIFT
\name:
    .rep PAGE_SIZE
        .byte 0
    .endr
.endm

DEFINE_PAGE __page_kernel_level_3
DEFINE_PAGE __page_init_id_lvl2
DEFINE_PAGE __page_kernel_lvl2
DEFINE_PAGE __page_stack_lvl1
