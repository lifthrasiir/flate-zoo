/* params: level (0-12) */
#include "fz.h"
#include "libdeflate.h"

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 1) return FZ_EPARAM;
    struct libdeflate_compressor *c = libdeflate_alloc_compressor((int)params[0]);
    if (!c) return FZ_EPARAM;
    size_t cap = libdeflate_deflate_compress_bound(c, in_len);
    unsigned char *buf = malloc(cap ? cap : 1);
    if (!buf) { libdeflate_free_compressor(c); return FZ_ENOMEM; }
    size_t n = libdeflate_deflate_compress(c, in, in_len, buf, cap);
    libdeflate_free_compressor(c);
    if (!n) { free(buf); return FZ_ELIB; }
    *out = buf;
    *out_len = n;
    return FZ_OK;
}
