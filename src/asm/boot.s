.section .text.entry
.global _start

_start:
    la sp, __stack_top

    call entry

1:
    wfi
    j 1b
