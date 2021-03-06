#include "string.h"

size_t strlen(const char *s) {
    size_t i;
    for (i = 0; s[i] != '\0'; i++);
    return i;
}

char* strcpy(char* s1, const char* s2)
{
    char* original = s1;

    while (*s2 != '\0')
        *s1++ = *s2++;
    *s1 = '\0';

    return original;
}

char* strncpy(char* s1, const char* s2, uint32_t n)
{
    char* original = s1;

    unsigned int i = 0;
    while (*s2 != '\0' && i < n) {
        *s1++ = *s2++;
        ++i;
    }
    *s1 = '\0';

    return original;
}

void* memcpy(void* buf1, const void* buf2, uint32_t bytes)
{
    uint8_t* buf_dst = buf1;
    const uint8_t* buf_src = buf2;

    for (unsigned int i = 0; i < bytes; ++i) {
        *buf_dst++ = *buf_src++;
    }

    return buf_dst;
}

void* memset(void* buf1, uint8_t value, size_t bytes)
{
    unsigned char* p = buf1;
    while(bytes--)
        *p++ = value;
    return p;
}

int memcmp(const void* s1, const void* s2, size_t n)
{
    const unsigned char *p1 = s1, *p2 = s2;
    while(n--)
        if( *p1 != *p2 )
            return *p1 - *p2;
        else
            p1++,p2++;
    return 0;
}

int strcmp(const char* s1, const char* s2)
{
    while (1) {
        if (*s1 != *s2)
            return (*s1 - *s2);
        if (*s1 == '\0')
            return (0);
        s1++;
        s2++;
    }
}

int strncmp(const char* s1, const char* s2, uint32_t n)
{
    for (unsigned int i = 0; i < n; ++i) {
        if (*s1 != *s2)
            return (*s1 - *s2);
        if (*s1 == '\0')
            return (0);
        s1++;
        s2++;
    }

    return 0;
}

char* strcat(char* s1, const char* s2)
{
    char* original = s1;

    while (*s1 != '\0')
        s1++;
    while (*s2 != '\0')
        *s1++ = *s2++;
    *s1 = '\0';

    return original;
}

char* strext(char* buf, const char* str, char sym)
{
    while (*str != '\0') {
        *buf++ = *str++;
        *buf++ = sym;
    }

    return buf;
}

int strspn(char* str, const char* accept)
{
    int len = strlen(accept);
    int i;

    for (i = 0; str[i] != '\0'; ++i) {
        bool is_found = false;

        for (int j = 0; j < len; ++j) {
            if (accept[j] == str[i]) {
                is_found = true;
                break;
            }
        }

        if (!is_found) {
            break;
        }
    }

    return i;
}

int strcspn(char* str, const char* rejected)
{
    int len = strlen(rejected);
    int i;

    for (i = 0; str[i] != '\0'; ++i) {
        bool is_not_found = true;

        for (int j = 0; j < len; ++j) {
            if (rejected[j] == str[i]) {
                is_not_found = false;
                break;
            }
        }

        if (!is_not_found) {
            break;
        }
    }

    return i;
}

char* strchr(const char* str, char ch)
{
    char* ptr = (char*)str;
    int len = strlen(str);

    for (int i = 0; i < len && *ptr != '\0'; ++i, ptr++) {
        if (*ptr == ch) {
            return ptr;
        }
    }

    return NULL;
}

char* strtok_r(char* str, const char* delim, char** save_ptr)
{
    char* end;

    if (str == NULL) {
        str = *save_ptr;
    }

    if (*str == '\0') {
        *save_ptr = str;
        return NULL;
    }

    /* scan leading delimiters */
    str += strspn(str, delim);
    if (*str == '\0') {
        *save_ptr = str;
        return NULL;
    }

    /* find the end of the token */
    end = str + strcspn(str, delim);
    if (*end == '\0') {
        *save_ptr = end;
        return str;
    }

    /* terminate the token */
    *end = '\0';
    *save_ptr = end + 1;
    return str;
}

char* memext(void* buff_dst, uint32_t n, const void* buff_src, char sym)
{
    uint8_t* buff_dst_ptr = buff_dst;
    uint8_t* buff_src_ptr = (uint8_t*)buff_src;

    for (unsigned int i = 0; i < n; ++i) {
        *buff_dst_ptr++ = *buff_src_ptr++;
        *buff_dst_ptr++ = sym;
    }

    return buff_dst;
}

/*
char* itoa(uint32_t value, char* str, uint32_t base)
{
    char* original = str;
    char digit;

    do {
        digit = value % base;
        value = value / base;
        if (digit < 10) {
            *str++ = digit | 0x30; //number
        } else if (digit < 16) {
            *str++ = ((digit - 10) | 0x40) + 1; //alpha
        } else {
            *str++ = '?';
        }
    } while (value > 0);

    if (base == 16) {
        //hexedecimal integer
        *str++ = 'x';
        *str++ = '0';
    } else if (base == 8) {
        //octal integer
        *str++ = 'o';
        *str++ = '0';
    } else if (base == 2) {
        //binary integer
        *str++ = 'b';
        *str++ = '0';
    }
    *str++ = '\0';

    strinv(original);

    return str;
}
*/

unsigned int atou(char* str)
{
    int k = 0;

    while (*str) {
        k = (k << 3) + (k << 1) + (*str) - '0';
        str++;
    }

    return k;
}

char* strinv(char* str)
{
    int i;
    uint32_t n = strlen(str);
    char buf[n + 2];
    char* cur = buf;

    for (i = n - 1; i >= 0; --i) {
        *cur++ = str[i];
    }
    *cur++ = '\0';

    strcpy(str, buf);

    return str;
}

//for printf
void _putchar(char character)
{
  // send char to console etc.
}
