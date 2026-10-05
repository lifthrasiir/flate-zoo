/* Minimal config.h for building GNU gzip's deflate.c/trees.c/bits.c outside its build system. */
#ifndef FZ_GZIP_CONFIG_H
#define FZ_GZIP_CONFIG_H
#include <stdbool.h>
#include <stddef.h>
#include <limits.h>
#include <assert.h>
#ifndef _Noreturn
# define _Noreturn
#endif
#define _GL_ATTRIBUTE_CONST __attribute__((__const__))
#define _GL_ATTRIBUTE_PURE __attribute__((__pure__))
#define OS_CODE 0x03
#endif
