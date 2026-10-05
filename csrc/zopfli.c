/* params: iterations, block_splitting (0/1), block_splitting_max */
#include "fz.h"
#include "zopfli/zopfli.h"

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 3) return FZ_EPARAM;
    ZopfliOptions o;
    ZopfliInitOptions(&o);
    o.numiterations = (int)params[0];
    o.blocksplitting = (int)params[1];
    o.blocksplittingmax = (int)params[2];
    unsigned char *buf = NULL;
    size_t n = 0;
    ZopfliCompress(&o, ZOPFLI_FORMAT_DEFLATE, in, in_len, &buf, &n);
    if (!buf) return FZ_ENOMEM;
    *out = buf;
    *out_len = n;
    return FZ_OK;
}
