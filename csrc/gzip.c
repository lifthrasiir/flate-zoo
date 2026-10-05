/* GNU gzip 1.14 deflate (deflate.c, trees.c, bits.c) driven from memory.
 * params: level, rsync
 * The output is what `gzip -N [--rsyncable]` writes between the gzip header and trailer. */
#include "fz.h"
#include <config.h>
#include <stdio.h>
#include "tailor.h"
#include "gzip.h"

extern int fz_gzip_mem_read(char *, unsigned);
extern void fz_gzip_begin(const unsigned char *, size_t);
extern unsigned char *fz_gzip_obuf;
extern size_t fz_gzip_olen;
extern int fz_gzip_oerr;

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 2 || params[0] < 1 || params[0] > 9) return FZ_EPARAM;
    fz_gzip_begin(in, in_len);
    level = (int)params[0];
    rsync = params[1] != 0;
    ush attr = 0;
    int meth = DEFLATED;
    bi_init(NO_FILE);
    read_buf = fz_gzip_mem_read;
    ct_init(&attr, &meth);
    gzip_deflate(level);
    flush_outbuf();
    if (fz_gzip_oerr) { free(fz_gzip_obuf); return FZ_ENOMEM; }
    if (!fz_gzip_obuf) fz_gzip_obuf = malloc(1);
    if (!fz_gzip_obuf) return FZ_ENOMEM;
    *out = fz_gzip_obuf;
    *out_len = fz_gzip_olen;
    return FZ_OK;
}
