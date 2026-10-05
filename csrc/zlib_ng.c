/* zlib-ng native API; otherwise identical to zlib_family.c.
 * params: level, window_bits, mem_level, strategy, inbuf, outbuf.
 *
 * The call pattern is that of zlib's examples/zpipe.c: input is handed over `inbuf` bytes at a
 * time and deflate() is called until it leaves output space unused, with `outbuf` bytes of
 * output space per call. 0 means everything at once, i.e. what compress2() does. Only stored
 * blocks (level 0) depend on this, since deflate_stored() sizes them after avail_in/avail_out. */
#include "fz.h"
#include "zlib-ng.h"

#define FZ_MAX_CHUNK 0x40000000u

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 6) return FZ_EPARAM;
    size_t inbuf = params[4] > 0 && params[4] < FZ_MAX_CHUNK ? (size_t)params[4] : FZ_MAX_CHUNK;
    size_t outbuf = params[5] > 0 && params[5] < FZ_MAX_CHUNK ? (size_t)params[5] : 0;
    zng_stream s;
    memset(&s, 0, sizeof s);
    if (zng_deflateInit2(&s, (int)params[0], Z_DEFLATED, -(int)params[1], (int)params[2], (int)params[3]) != Z_OK)
        return FZ_EPARAM;
    size_t cap = zng_deflateBound(&s, in_len) + 64, len = 0, pos = 0;
    unsigned char *buf = malloc(cap);
    int ret = Z_OK, flush;
    do {
        size_t n = in_len - pos < inbuf ? in_len - pos : inbuf;
        s.next_in = in + pos;
        s.avail_in = (uint32_t)n;
        pos += n;
        flush = pos == in_len ? Z_FINISH : Z_NO_FLUSH;
        do {
            size_t want = outbuf ? outbuf : FZ_MAX_CHUNK;
            if (buf && cap - len < want && (outbuf || cap == len)) {
                size_t ncap = cap * 2 > len + want ? cap * 2 : len + want;
                if (!outbuf) ncap = cap * 2;
                unsigned char *nbuf = realloc(buf, ncap);
                if (!nbuf) { free(buf); buf = NULL; }
                buf = nbuf;
                cap = ncap;
            }
            if (!buf) { zng_deflateEnd(&s); return FZ_ENOMEM; }
            uint32_t avail = (uint32_t)(cap - len < want ? cap - len : want);
            s.next_out = buf + len;
            s.avail_out = avail;
            ret = zng_deflate(&s, flush);
            if (ret == Z_STREAM_ERROR) { zng_deflateEnd(&s); free(buf); return FZ_ELIB; }
            len += avail - s.avail_out;
        } while (s.avail_out == 0);
    } while (flush != Z_FINISH);
    zng_deflateEnd(&s);
    if (ret != Z_STREAM_END) { free(buf); return FZ_ELIB; }
    *out = buf;
    *out_len = len;
    return FZ_OK;
}
