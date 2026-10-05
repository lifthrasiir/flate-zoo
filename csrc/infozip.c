/* Info-ZIP Zip 3.0 deflate (deflate.c, trees.c) driven from memory.
 * params: level
 * The output is the data of a deflated entry as `zip -N` writes it. */
#include "fz.h"

extern int fz_infozip_compress(const unsigned char *, size_t, int, unsigned char **, size_t *);

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 1 || params[0] < 1 || params[0] > 9) return FZ_EPARAM;
    return fz_infozip_compress(in, in_len, (int)params[0], out, out_len) ? FZ_ENOMEM : FZ_OK;
}
