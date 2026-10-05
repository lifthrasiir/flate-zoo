/* Replacements for the pieces of zip.c/zipup.c/globals.c/util.c that Info-ZIP 3.0's deflate.c and
 * trees.c rely on. Mirrors zipup.c's filecompress(): the entry is deflated from memory into a
 * growing memory buffer. `seekable()` and `use_descriptors` are set as when zip streams to a pipe
 * (`zip -N - file`), which keeps zip from replacing an incompressible entry by a stored one, so
 * that the output is always a DEFLATE stream. */
#include "zip.h"
#include <stdlib.h>
#include <string.h>

int level, verbose, noisy, use_descriptors, display_globaldots, mesg_line_started;
zoff_t dot_size, dot_count;
char *key;
FILE *mesg;
extern ulg window_size;
unsigned (*read_buf)(char *buf, unsigned size);

static const unsigned char *in_p;
static size_t in_left;
static unsigned char *obuf;
static size_t olen, ocap;
static int oerr;
static char file_outbuf[16384];

static unsigned mem_read(char *b, unsigned bsize) {
    size_t n = in_left < bsize ? in_left : bsize;
    memcpy(b, in_p, n);
    in_p += n;
    in_left -= n;
    return (unsigned)n;
}

void flush_outbuf(char *o_buf, unsigned *o_idx) {
    if (*o_idx && !oerr) {
        if (olen + *o_idx > ocap) {
            size_t cap = ocap ? ocap * 2 : 65536;
            while (cap < olen + *o_idx) cap *= 2;
            unsigned char *p = realloc(obuf, cap);
            if (!p) oerr = 1;
            else { obuf = p; ocap = cap; }
        }
        if (!oerr) memcpy(obuf + olen, o_buf, *o_idx);
        olen += *o_idx;
    }
    *o_idx = 0;
}

int seekable(void) { return 0; }
void error(ZCONST char *h) { fprintf(stderr, "infozip: %s\n", h); abort(); }

/* Returns 0 on success, -1 on allocation failure. */
int fz_infozip_compress(const unsigned char *in, size_t len, int lvl, unsigned char **out, size_t *out_len) {
    ush att = UNKNOWN, flags = 0;
    int meth = DEFLATE;
    level = lvl;
    verbose = noisy = use_descriptors = display_globaldots = 0;
    dot_size = dot_count = 0;
    key = NULL;
    obuf = NULL; olen = ocap = 0; oerr = 0;
    in_p = in; in_left = len;
    read_buf = mem_read;
    window_size = 0L;
    bi_init(file_outbuf, sizeof file_outbuf, TRUE);
    ct_init(&att, &meth);
    lm_init(level, &flags);
    deflate();
    if (oerr) { free(obuf); return -1; }
    if (!obuf) obuf = malloc(1);
    if (!obuf) return -1;
    *out = obuf;
    *out_len = olen;
    return 0;
}
