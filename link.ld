/*
*  link.ld
*/
OUTPUT_FORMAT(elf32-i386)
ENTRY (start)
SECTIONS {
    . = 0x00100000;
    startkernel = .;
    .text : {
	  text_start = .;
    *(.text)
  }

.rodata ALIGN (0x1000) : {
    *(.rodata)
  }

.data ALIGN (0x1000) : {
    *(.data)
    end_data = .;
  }


.bss : {
    sbss = .;
    *(COMMON)
    *(.bss)
    ebss = .;
    endkernel = .;
  }
}
