/* params: btype, use_lz77, windowsize, minmatch, nicematch, lazymatching */
#define LODEPNG_NO_COMPILE_DISK
#define LODEPNG_NO_COMPILE_CPP
#include "fz.h"
#include "lodepng.h"

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 6) return FZ_EPARAM;
    LodePNGCompressSettings s;
    lodepng_compress_settings_init(&s);
    s.btype = (unsigned)params[0];
    s.use_lz77 = (unsigned)params[1];
    s.windowsize = (unsigned)params[2];
    s.minmatch = (unsigned)params[3];
    s.nicematch = (unsigned)params[4];
    s.lazymatching = (unsigned)params[5];
    unsigned char *buf = NULL;
    size_t n = 0;
    unsigned err = lodepng_deflate(&buf, &n, in, in_len, &s);
    if (err) { free(buf); return FZ_ELIB; }
    if (!buf) buf = malloc(1);
    *out = buf;
    *out_len = n;
    return FZ_OK;
}
