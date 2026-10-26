/* params: level. Go's compress/flate and klauspost/compress/flate, via the wasm2c translation
 * of go/wasm that fetch.sh generates into vendor/go-wasm2c. Every call gets a fresh instance
 * (well under a millisecond), so no Go heap or runtime state survives between calls and
 * concurrent calls need no locking. */
#include "fz.h"
#include "go/wasi.h"
#include "wasm-rt-exceptions.h"
#include "wasm-rt-impl.h"

enum { LIB_STD = 0, LIB_KLAUSPOST = 1 };

static int run(u32 lib, FZ_ARGS) {
    if (nparams != 1) return FZ_EPARAM;
    /* the input, its copy in the window and the output all live in the 4 GiB wasm32 heap */
    if (in_len > 0x40000000) return FZ_ENOMEM;
    w2c_fz__go *m = calloc(1, sizeof *m);
    if (!m) return FZ_ENOMEM;
    struct w2c_wasi__snapshot__preview1 env = { m, 1000000000 };
    wasm2c_fz__go_instantiate(m, &env);
    int rc;
    if (wasm_rt_impl_try() == WASM_RT_TRAP_NONE) {
        w2c_fz__go_0x5Finitialize(m);
        u32 src = w2c_fz__go_fz_alloc(m, (u32)in_len);
        if ((u64)src + in_len > m->w2c_memory.size) wasm_rt_trap(WASM_RT_TRAP_OOB);
        memcpy(m->w2c_memory.data + src, in, in_len);
        u32 dst = w2c_fz__go_fz_run(m, lib, src, (u32)in_len, (u32)(s32)params[0]);
        u32 n = w2c_fz__go_fz_out_len(m);
        if (!dst) {
            rc = FZ_EPARAM;
        } else if ((u64)dst + n > m->w2c_memory.size) {
            rc = FZ_ELIB;
        } else if (!(*out = malloc(n + 1))) {
            rc = FZ_ENOMEM;
        } else {
            memcpy(*out, m->w2c_memory.data + dst, n);
            *out_len = n;
            rc = FZ_OK;
        }
    } else {
        rc = FZ_ELIB;
    }
    wasm2c_fz__go_free(m);
    free(m);
    return rc;
}

int FZ_ENTRY(std_run)(FZ_ARGS) { return run(LIB_STD, in, in_len, params, nparams, out, out_len); }
int FZ_ENTRY(klauspost_run)(FZ_ARGS) {
    return run(LIB_KLAUSPOST, in, in_len, params, nparams, out, out_len);
}
