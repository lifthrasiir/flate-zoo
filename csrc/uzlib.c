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
    /* uzlib_compress computes `in + in_len - 3` up front, which wraps around for short inputs
     * at low addresses (e.g. Rust's dangling pointer for an empty slice) and then reads from it.
     * Such inputs are all literals anyway. */
    if (in_len < 3) {
        for (size_t i = 0; i < in_len; ++i) zlib_literal(&c, in[i]);
    } else {
        uzlib_compress(&c, in, (unsigned)in_len);
    }
    zlib_finish_block(&c);
    free(c.hash_table);
    if (!c.outbuf) c.outbuf = malloc(1);
    if (!c.outbuf) return FZ_ENOMEM;
    *out = c.outbuf;
    *out_len = (size_t)c.outlen;
    return FZ_OK;
}
