/* params: level (0 = stored, 1 = compressed) */
#include "fz.h"
#include "slz.h"

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 1) return FZ_EPARAM;
    struct slz_stream s;
    /* worst case is stored blocks: 5 bytes per 65535, plus room for SLZ's direct enqueueing */
    size_t cap = in_len + in_len / 8192 * 5 + 64;
    unsigned char *buf = malloc(cap);
    if (!buf) return FZ_ENOMEM;
    if (slz_rfc1951_init(&s, (int)params[0]) < 0) { free(buf); return FZ_EPARAM; }
    size_t n = 0;
    for (size_t pos = 0; pos < in_len;) { /* slz takes `long` lengths */
        long chunk = in_len - pos > 0x10000000 ? 0x10000000 : (long)(in_len - pos);
        n += (size_t)slz_rfc1951_encode(&s, buf + n, in + pos, chunk, pos + (size_t)chunk < in_len);
        pos += (size_t)chunk;
    }
    n += (size_t)slz_rfc1951_finish(&s, buf + n);
    *out = buf;
    *out_len = n;
    return FZ_OK;
}
