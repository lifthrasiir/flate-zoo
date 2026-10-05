/* ISA-L igzip (portable C "base" build), raw DEFLATE output.
 * params: level (0..3), chunk_kib (input bytes handed to each isal_deflate call, 1024 = what
 * the igzip CLI does; ignored when stateless), stateless (1 = one isal_deflate_stateless call). */
#include "fz.h"
#include "igzip_lib.h"

static const size_t lvl_buf_size[4] = {ISAL_DEF_LVL0_DEFAULT, ISAL_DEF_LVL1_DEFAULT,
                                       ISAL_DEF_LVL2_DEFAULT, ISAL_DEF_LVL3_DEFAULT};

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 3 || params[0] < 0 || params[0] > 3 || params[1] < 1 || params[1] > (1 << 20))
        return FZ_EPARAM;
    if (in_len > 0xffffffffu) return FZ_EPARAM;
    int level = (int)params[0];
    size_t chunk = (size_t)params[1] * 1024;
    int stateless = params[2] != 0;

    struct isal_zstream s;
    uint8_t *lb = malloc(lvl_buf_size[level]);
    if (!lb) return FZ_ENOMEM;
    isal_deflate_init(&s);
    s.level = level;
    s.level_buf = lb;
    s.level_buf_size = lvl_buf_size[level];
    s.gzip_flag = IGZIP_DEFLATE;
    s.flush = NO_FLUSH;

    int rc = FZ_ELIB;
    size_t cap = in_len + in_len / 2 + 4096, len = 0;
    uint8_t *buf = malloc(cap);
    if (!buf) { free(lb); return FZ_ENOMEM; }

    if (stateless) {
        isal_deflate_stateless_init(&s);
        s.level = level;
        s.level_buf = lb;
        s.level_buf_size = lvl_buf_size[level];
        s.gzip_flag = IGZIP_DEFLATE;
        s.flush = NO_FLUSH;
        s.next_in = (uint8_t *)in;
        s.avail_in = (uint32_t)in_len;
        s.end_of_stream = 1;
        s.next_out = buf;
        s.avail_out = (uint32_t)cap;
        if (isal_deflate_stateless(&s) != COMP_OK || s.avail_in != 0) goto done;
        len = s.total_out;
    } else {
        size_t pos = 0;
        for (;;) {
            if (s.avail_in == 0) {
                size_t n = in_len - pos < chunk ? in_len - pos : chunk;
                s.next_in = (uint8_t *)in + pos;
                s.avail_in = (uint32_t)n;
                pos += n;
                s.end_of_stream = pos >= in_len;
            }
            if (cap - len < 65536) {
                uint8_t *nb = realloc(buf, cap * 2);
                if (!nb) { rc = FZ_ENOMEM; goto done; }
                buf = nb;
                cap *= 2;
            }
            s.next_out = buf + len;
            s.avail_out = (uint32_t)(cap - len);
            if (isal_deflate(&s) != COMP_OK) goto done;
            len = (size_t)(s.next_out - buf);
            if (s.avail_in == 0 && pos >= in_len && s.avail_out != 0) break;
        }
    }
    if (len == 0) goto done;
    *out = buf;
    *out_len = len;
    buf = NULL;
    rc = FZ_OK;
done:
    free(buf);
    free(lb);
    return rc;
}
