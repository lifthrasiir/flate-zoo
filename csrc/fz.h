/* Common declarations for the encoder shims.
 *
 * Every entry point has the `fz_fn` signature: it compresses `in` into a raw DEFLATE stream
 * (RFC 1951, no zlib/gzip wrapper) stored in a malloc'ed buffer that the caller frees.
 * `params` are the backend parameters in the order declared on the Rust side. */
#ifndef FZ_H
#define FZ_H

#include <stddef.h>
#include <stdlib.h>
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

enum { FZ_OK = 0, FZ_EPARAM = -1, FZ_ENOMEM = -2, FZ_ELIB = -3 };

#define FZ_CAT_(a, b) a##_##b
#define FZ_CAT(a, b) FZ_CAT_(a, b)
/* FZ_ENTRY(run) expands to fz_<unit>_run; FZ_UNIT is defined by build.rs. */
#define FZ_ENTRY(name) FZ_CAT(FZ_CAT(fz, FZ_UNIT), name)
#define FZ_ARGS const unsigned char *in, size_t in_len, const long long *params, size_t nparams, \
                unsigned char **out, size_t *out_len

/* Strips the 2-byte zlib header and the 4-byte Adler-32 trailer, transferring ownership of
 * `z` (malloc'ed, `zlen` bytes) to `*out`. */
static inline int fz_from_zlib(unsigned char *z, size_t zlen, unsigned char **out, size_t *out_len) {
    if (!z) return FZ_ENOMEM;
    if (zlen < 6 || (z[1] & 0x20)) { free(z); return FZ_ELIB; }
    memmove(z, z + 2, zlen - 6);
    *out = z;
    *out_len = zlen - 6;
    return FZ_OK;
}

#ifdef __cplusplus
}
#endif
#endif
