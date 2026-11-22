OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80200000;

MAX_CPUS = 32;
STACK_SIZE = 0x4000; /* 16 KiB */

SECTIONS
{
  . = BASE_ADDRESS;
  __kernel_start = .;

  .text : {
    KEEP(*(.text.entry))
    *(.text .text.*)
  }

  /* Page align whatever gets different page permissions. */
  . = ALIGN(4096);
  __text_end = .;

  .rodata : {
    *(.srodata .srodata.*)
    *(.rodata .rodata.*)
  }

  . = ALIGN(4096);
  __rodata_end = .;

  .data : {
    PROVIDE(__global_pointer$ = . + 0x800);

    KEEP(*(.data.boot))
    *(.sdata .sdata.*)
    *(.data .data.*)
  }

  . = ALIGN(16);
  .bss (NOLOAD) : {
    __bss_start = .;
    *(.sbss .sbss.*)
    *(.bss .bss.*)
    *(COMMON)
    . = ALIGN(16);
    __bss_end = .;
  }

  . = ALIGN(4096);
  .boot_stacks (NOLOAD) : {
    __boot_stacks_start = .;
    . += STACK_SIZE * MAX_CPUS;
    __boot_stacks_end = .;
  }

  __stack_top = __boot_stacks_end;

  . = ALIGN(4096);
  __kernel_end = .;
}
