#ifndef _INLINEASSEMBLY_H_
#define _INLINEASSEMBLY_H_

#include "../../i386-elf-4.9.1-Linux-x86_64/lib/gcc/i386-elf/4.9.1/include/stddef.h"
#include "../../i386-elf-4.9.1-Linux-x86_64/lib/gcc/i386-elf/4.9.1/include/stdint.h"
#include "../../i386-elf-4.9.1-Linux-x86_64/lib/gcc/i386-elf/4.9.1/include/stdbool.h"
#include "../lib/string.h"

//----Memory access----

//FAR_PEEKx
extern uint32_t farpeekl(uint16_t sel, void* off);

//FAR_POKEx
extern void farpokeb(uint16_t sel, void* off, uint8_t v);

//I/O access
extern void outb(uint16_t port, uint8_t val);

extern uint8_t inb(uint16_t port);

//my
extern uint16_t inw( uint16_t p_port);

extern void outw (uint16_t p_port,uint16_t p_data);

//IO_WAIT
extern void io_wait(void);

//----Interrupt-related functions----

//Interrupts Enabled?
extern bool are_interrupts_enabled();

//Push/pop interrupt flag
extern unsigned long save_irqdisable(void);

extern void irqrestore(unsigned long flags);

//extern void intended_usage(void);

//LIDT
extern void lidt(void* base, uint16_t size);

//----CPU-related functions----

//CPUID
extern void cpuid(int code, uint32_t* a, uint32_t* d);

extern int cpuid_string(int code, uint32_t where[4]);

#endif
