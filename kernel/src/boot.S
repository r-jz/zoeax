.equ PAGE_SHIFT, 12
.equ PTE_VALID, 1 << 0
.equ PTE_READ, 1 << 1
.equ PTE_WRITE, 1 << 2
.equ PTE_EXECUTE, 1 << 3

.section ".boot.text", "ax"
.global boot


boot:
	.option push
	.option norelax
1:
	auipc ra, %pcrel_hi(1f)
	ld ra, %pcrel_lo(1b)(ra)
	jr ra
	.align 3
1:
	.dword _abs_start
	.option pop

_abs_start:
    .option norelax

.macro LA_FAR, reg, sym
	.option push
	.option norelax
.Lla_\@:
	auipc \reg, %pcrel_hi(.Lla_ptr_\@)
	ld \reg, %pcrel_lo(.Lla_\@)(\reg)
	j .Lla_end_\@
	.align 3
.Lla_ptr_\@:
	.dword \sym
.Lla_end_\@:
	.option pop
.endm

.macro PPN, reg, pt
	la \reg, \pt
	srli \reg, \reg, PAGE_SHIFT
.endm

/* See _PTE_SET. va is a symbol */
.macro PTE_SET, pt, va, lvl, ppn, flags
	la t1, \va
	_PTE_SET \pt, \lvl, \ppn, \flags
.endm

/* See _PTE_SET. va is a far symbol */
.macro PTE_SET_FAR, pt, va, lvl, ppn, flags
	LA_FAR t1, \va
	_PTE_SET \pt, \lvl, \ppn, \flags
.endm

/* Clobbers: \ppn t0 t1
 * va must be present in t1
 * Arguments:
 *   pt: symbol
 *   va: symbol
 *   lvl: immediate
 *   ppn: register
 *   flags: immediate
 */
.macro _PTE_SET, pt, lvl, ppn, flags
	/* extract the correct level from \va */
	srli t1, t1, (12 + 9 * \lvl)
	andi t1, t1, (1 << 9) - 1
	slli t1, t1, 3

	/* get index in the pte */
	la t0, \pt
	add t0, t0, t1

	/* Transform the PPN into the PTE */
	slli \ppn, \ppn, 10
	addi \ppn, \ppn, \flags

	sd \ppn, 0(t0)
.endm

	csrw sie, 0
	csrw sip, 0

	/* Setup identity mapping */
	PPN t2, __page_init_id_lvl2
	PTE_SET __page_kernel_level_3, _RAM_START, 3, t2, PTE_VALID

	/* 1GB identity mapping */
	PPN t2, _RAM_START
	PTE_SET __page_init_id_lvl2, _RAM_START, 2, t2, PTE_VALID | PTE_EXECUTE | PTE_READ | PTE_WRITE

	PPN t2, __page_kernel_lvl2
	PTE_SET_FAR __page_kernel_level_3, _KERNEL_CODE_VIRTUAL, 3, t2, PTE_VALID

	/* 1GB kernel mapping */
	PPN t2, _RAM_START
	PTE_SET_FAR __page_kernel_lvl2, _KERNEL_CODE_VIRTUAL, 2, t2, PTE_VALID | PTE_EXECUTE | PTE_READ | PTE_WRITE

	/* 2MB kernel stack mapping */
#	#PPN t2, __page_stack_lvl1
	# PTE_SET_FAR __page_kernel_lvl2, __stack_bottom, 2, t2, PTE_VALID
	#
	# PPN t2, __stack_bottom_phyis 
	# PTE_SET_FAR __page_stack_lvl1, __stack_bottom, 1, t2, PTE_VALID | PTE_READ | PTE_WRITE

	li t1, 9
	slli t1, t1, 60
	PPN t0, __page_kernel_level_3
	or t0, t0, t1
	sfence.vma x0, x0
	csrw satp, t0
	sfence.vma x0, x0



	li x1, 0
	li x2, 0
	li x3, 0
	li x4, 0
	li x5, 0
	li x6, 0
	li x7, 0
	li x8, 0
	li x9, 0
	/* skip a0,a1,a2 (x10,x11,x12) */
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

.Ltmp2: auipc a2, %pcrel_hi(__free_ram_phys)
        addi  a2, a2, %pcrel_lo(.Ltmp2)
.Ltmp3: auipc a3, %pcrel_hi(__free_ram_end_phys)
        addi  a3, a3, %pcrel_lo(.Ltmp3)

	/* setup global pointer (see .data in hades.x) */
	LA_FAR gp, __global_pointer$
	/* setup virtual address stack (see .stack in hades.x) */
	LA_FAR sp, __stack_top
	/* jump to main in virtual addresses */
	LA_FAR a4, _kernel_main
	jalr zero, 0(a4)

.macro DEFINE_PAGE, name

.align PAGE_SHIFT
\name: 
.rep (1 << PAGE_SHIFT)
	.byte 0
.endr

.endm

DEFINE_PAGE __page_kernel_level_3
DEFINE_PAGE __page_init_id_lvl2
DEFINE_PAGE __page_kernel_lvl2
DEFINE_PAGE __page_stack_lvl1
