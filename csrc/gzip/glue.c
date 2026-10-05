/* Replacements for the pieces of gzip.c/util.c/zip.c that deflate.c, trees.c and bits.c rely on.
 * Compression runs from an in-memory buffer into a growing in-memory buffer. */
#include <config.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tailor.h"
#include "gzip.h"

uch inbuf[INBUFSIZ + INBUF_EXTRA];
uch outbuf[OUTBUFSIZ + OUTBUF_EXTRA];
ush d_buf[DIST_BUFSIZE];
uch window[2L * WSIZE];
ush prev[1L << 16];

int method;
unsigned insize, inptr, outcnt;
off_t bytes_in, bytes_out, header_bytes;
int rsync;
int ifd, ofd;
int exit_code, quiet, level, test, to_stdout, save_orig_name;
char ifname[1024], ofname[1024];
char *program_name = "gzip";
struct timespec time_stamp;
off_t ifile_size;

/* Input and output cursors of the current run. */
static const unsigned char *src_p;
static size_t src_left;
unsigned char *fz_gzip_obuf;
size_t fz_gzip_olen, fz_gzip_ocap;
int fz_gzip_oerr;

int fz_gzip_mem_read(char *buf, unsigned size) {
    size_t n = src_left < size ? src_left : size;
    memcpy(buf, src_p, n);
    src_p += n;
    src_left -= n;
    bytes_in += (off_t)n;
    return (int)n;
}

void fz_gzip_begin(const unsigned char *in, size_t len) {
    src_p = in;
    src_left = len;
    bytes_in = bytes_out = header_bytes = 0;
    insize = inptr = outcnt = 0;
    fz_gzip_obuf = NULL;
    fz_gzip_olen = fz_gzip_ocap = 0;
    fz_gzip_oerr = 0;
}

void flush_outbuf(void) {
    if (outcnt == 0) return;
    if (!fz_gzip_oerr) {
        if (fz_gzip_olen + outcnt > fz_gzip_ocap) {
            size_t cap = fz_gzip_ocap ? fz_gzip_ocap * 2 : 65536;
            while (cap < fz_gzip_olen + outcnt) cap *= 2;
            unsigned char *p = realloc(fz_gzip_obuf, cap);
            if (!p) fz_gzip_oerr = 1;
            else { fz_gzip_obuf = p; fz_gzip_ocap = cap; }
        }
        if (!fz_gzip_oerr) memcpy(fz_gzip_obuf + fz_gzip_olen, outbuf, outcnt);
    }
    fz_gzip_olen += outcnt;
    bytes_out += outcnt;
    outcnt = 0;
}

void flush_window(void) {}
int fill_inbuf(int eof_ok) { (void)eof_ok; return -1; }
ulg updcrc(const uch *s, unsigned n) { (void)s; (void)n; return 0; }
ulg getcrc(void) { return 0; }
int file_read(char *buf, unsigned size) { return fz_gzip_mem_read(buf, size); }
void gzip_error(char const *m) { fprintf(stderr, "gzip: %s\n", m); abort(); }
void xalloc_die(void) { abort(); }
void warning(char const *m) { (void)m; }
void read_error(void) { abort(); }
void write_error(void) { abort(); }
