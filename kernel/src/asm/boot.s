.equ BOOT_MAX_CPUS, 32
.equ BOOT_STACK_SIZE, 16384
.equ BOOT_STACK_SHIFT, 14   /* log2(16384) */

.section .data.boot
.balign 8
.global __boot_stack_next
__boot_stack_next:
    .dword 0

.section .text.entry
.balign 4
.global _start
.type _start, @function

_start:
    /*
     *   satp = 0
     *   sstatus.SIE = 0
     *   a0 = hartid
     *   a1 = opaque
     *   other regs undefined
     */
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    /*
     * HSM-started harts should already have SIE cleared, but this makes sure it happens anw
     */
    csrw sie, zero
    csrci sstatus, 2

    /*
     * Allocate stack slot by arrival order
     * t2 receives the old value of __boot_stack_next
     */
    la      t0, __boot_stack_next
    li      t1, 1
    amoadd.d.aq t2, t1, (t0)

    li      t3, BOOT_MAX_CPUS
    bgeu    t2, t3, .Lno_stack

    la      t0, __boot_stacks_start
    slli    t2, t2, BOOT_STACK_SHIFT
    add     t0, t0, t2
    li      t1, BOOT_STACK_SIZE
    add     sp, t0, t1
    andi    sp, sp, -16

    tail    entry

.Lno_stack:
    wfi
    j       .Lno_stack

.size _start, . - _start
