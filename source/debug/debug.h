#ifndef _DEBUG_H_
#define _DEBUG_H_

//serial PORT debug

#include "../../i386-elf-4.9.1-Linux-x86_64/lib/gcc/i386-elf/4.9.1/include/stddef.h"
#include "../../i386-elf-4.9.1-Linux-x86_64/lib/gcc/i386-elf/4.9.1/include/stdint.h"
#include "../lib/string.h"
#include "../lib/printf.h"

#define PORT_COM1 0x3f8   /* COM1 */

extern void serial_init();

extern int serial_is_transmit_empty();

extern void serial_write_symbol(char ch);

extern void serial_write_str(const char* str);

//debug serial printf
//formats
//%c - char
//%i/%d - int32
//%u - uint32
//%ll - int64
//%llu - uint64
//%x - uint32 address
//%llx - uint64 address
//%s - string
extern void serial_printf(const char* fmt, ...);

#endif