/* Efficient Compression Tool's deflate, as used for its gzip/zip output (ZopfliGzipCompress).
 * params: level (1..9), iterations (0 = level default, else 10..9999), twice (0..9)
 * ECT's mode word is `twice * 10000 + (iterations ? iterations : level)`; level 1 is zlib -9. */
#include "fz.h"
#include "zopfli/zopfli.h"
#include "zopfli/deflate.h"
#include "zlib.h"

extern "C" int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 3) return FZ_EPARAM;
    long long level = params[0], iters = params[1], twice = params[2];
    if (level < 1 || level > 9 || (iters != 0 && (iters < 10 || iters > 9999)) || twice < 0 || twice > 9)
        return FZ_EPARAM;
    unsigned mode = (unsigned)(twice * 10000 + (iters ? iters : level));

    if (mode == 1) {
        /* ZopfliGzipCompress: zlib level 9 on ECT's own (Chromium-derived) zlib */
        z_stream s;
        memset(&s, 0, sizeof s);
        if (deflateInit2(&s, 9, Z_DEFLATED, -15, 8, Z_DEFAULT_STRATEGY) != Z_OK) return FZ_ELIB;
        size_t cap = deflateBound(&s, in_len) + 8;
        unsigned char *buf = (unsigned char *)malloc(cap);
        if (!buf) { deflateEnd(&s); return FZ_ENOMEM; }
        s.next_in = (z_const unsigned char *)in;
        s.avail_in = (uInt)in_len;
        s.next_out = buf;
        s.avail_out = (uInt)cap;
        int rc = deflate(&s, Z_FINISH);
        size_t n = s.total_out;
        deflateEnd(&s);
        if (rc != Z_STREAM_END) { free(buf); return FZ_ELIB; }
        *out = buf;
        *out_len = n;
        return FZ_OK;
    }

    /* ZopfliBuffer(mode, multithreading = 0, ...) */
    ZopfliOptions o;
    ZopfliInitOptions(&o, mode, 0, 0);
    unsigned char bp = 0;
    unsigned char *buf = NULL;
    size_t n = 0;
    ZopfliDeflate(&o, 1, in, in_len, &bp, &buf, &n);
    if (!buf) return FZ_ENOMEM;
    *out = buf;
    *out_len = n;
    return FZ_OK;
}
