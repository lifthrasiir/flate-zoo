/* params: hash_bits, dict_size */
#include "fz.h"
#include "uzlib.h"

int FZ_ENTRY(run)(FZ_ARGS) {
    if (nparams != 2 || in_len > 0x7fffffff) return FZ_EPARAM;
    struct uzlib_comp c;
    memset(&c, 0, sizeof c);
    c.hash_bits = (unsigned)params[0];
    c.dict_size = (unsigned)params[1];
    c.hash_table = calloc((size_t)1 << c.hash_bits, sizeof(uzlib_hash_entry_t));
    if (!c.hash_table) return FZ_ENOMEM;
    zlib_start_block(&c);
    uzlib_compress(&c, in, (unsigned)in_len);
    zlib_finish_block(&c);
    free(c.hash_table);
    if (!c.outbuf) c.outbuf = malloc(1);
    if (!c.outbuf) return FZ_ENOMEM;
    *out = c.outbuf;
    *out_len = (size_t)c.outlen;
    return FZ_OK;
}
