/* params: level (0-8) */
#include "fz.h"
#include "sdefl.h"

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 1 || in_len > 0x3fffffff) return FZ_EPARAM;
    struct sdefl *s = calloc(1, sizeof *s);
    unsigned char *buf = malloc((size_t)sdefl_bound((int)in_len) + 16);
    if (!s || !buf) { free(s); free(buf); return FZ_ENOMEM; }
    int n = sdeflate(s, buf, in, (int)in_len, (int)params[0]);
    free(s);
    *out = buf;
    *out_len = (size_t)n;
    return FZ_OK;
}
