/* params: level (0-10), strategy (zlib numbering) */
#include "fz.h"
#include "miniz.h"

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 2) return FZ_EPARAM;
    /* negative window bits: no zlib header */
    mz_uint flags = tdefl_create_comp_flags_from_zip_params((int)params[0], -15, (int)params[1]);
    size_t n = 0;
    unsigned char *buf = tdefl_compress_mem_to_heap(in, in_len, &n, (int)flags);
    if (!buf) return in_len ? FZ_ELIB : FZ_ENOMEM;
    *out = buf;
    *out_len = n;
    return FZ_OK;
}
