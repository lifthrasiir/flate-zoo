/* Just enough of WASI preview 1 for the Go runtime of go/wasm, and deterministic: the clock is
 * a per-instance counter, randomness is constant, and there are no args, environment or files.
 * Go's own output goes to stderr, which only happens on fatal errors. */
#include <stdio.h>
#include <string.h>
#include "wasi.h"

typedef struct w2c_wasi__snapshot__preview1 Env;

enum { ESUCCESS = 0, EBADF = 8, EINVAL = 28 };

/* Guest memory at [off, off+len); traps when out of bounds. */
static u8 *mem(Env *e, u32 off, u64 len) {
    wasm_rt_memory_t *m = &e->inst->w2c_memory;
    if ((u64)off + len > m->size) wasm_rt_trap(WASM_RT_TRAP_OOB);
    return m->data + off;
}
static u32 load32(Env *e, u32 off) { u32 v; memcpy(&v, mem(e, off, 4), 4); return v; }
static void store32(Env *e, u32 off, u32 v) { memcpy(mem(e, off, 4), &v, 4); }
static void store64(Env *e, u32 off, u64 v) { memcpy(mem(e, off, 8), &v, 8); }

#define WASI(name) w2c_wasi__snapshot__preview1_##name

u32 WASI(args_sizes_get)(Env *e, u32 argc, u32 size) {
    store32(e, argc, 0);
    store32(e, size, 0);
    return ESUCCESS;
}
u32 WASI(args_get)(Env *e, u32 argv, u32 buf) { return ESUCCESS; }
u32 WASI(environ_sizes_get)(Env *e, u32 count, u32 size) {
    store32(e, count, 0);
    store32(e, size, 0);
    return ESUCCESS;
}
u32 WASI(environ_get)(Env *e, u32 environ, u32 buf) { return ESUCCESS; }

/* Both clocks advance by 1us per reading. The runtime insists on a non-zero monotonic clock;
 * nothing in compress/flate looks at time. */
u32 WASI(clock_time_get)(Env *e, u32 id, u64 precision, u32 out) {
    e->clock += 1000;
    store64(e, out, e->clock);
    return ESUCCESS;
}
u32 WASI(random_get)(Env *e, u32 buf, u32 len) {
    memset(mem(e, buf, len), 0x5a, len);
    return ESUCCESS;
}

u32 WASI(fd_write)(Env *e, u32 fd, u32 iovs, u32 iovs_len, u32 nwritten) {
    if (fd != 1 && fd != 2) return EBADF;
    u32 total = 0;
    for (u32 i = 0; i < iovs_len; i++) {
        u32 buf = load32(e, iovs + 8 * i), len = load32(e, iovs + 8 * i + 4);
        fwrite(mem(e, buf, len), 1, len, stderr);
        total += len;
    }
    store32(e, nwritten, total);
    return ESUCCESS;
}
u32 WASI(fd_close)(Env *e, u32 fd) { return EBADF; }
u32 WASI(fd_fdstat_get)(Env *e, u32 fd, u32 stat) { return EBADF; }
u32 WASI(fd_fdstat_set_flags)(Env *e, u32 fd, u32 flags) { return EBADF; }
/* EBADF on fd 3 tells the runtime that there are no preopened directories. */
u32 WASI(fd_prestat_get)(Env *e, u32 fd, u32 prestat) { return EBADF; }
u32 WASI(fd_prestat_dir_name)(Env *e, u32 fd, u32 path, u32 len) { return EBADF; }

/* Only used for usleep. Every subscription fires at once, which the runtime accepts. */
u32 WASI(poll_oneoff)(Env *e, u32 subs, u32 events, u32 nsubs, u32 nevents) {
    if (nsubs == 0) return EINVAL;
    for (u32 i = 0; i < nsubs; i++) {
        u8 *s = mem(e, subs + 48 * i, 48), *ev = mem(e, events + 32 * i, 32);
        memset(ev, 0, 32);
        memcpy(ev, s, 8);  /* userdata */
        ev[10] = s[8];     /* event type = subscription tag */
    }
    store32(e, nevents, nsubs);
    return ESUCCESS;
}
u32 WASI(sched_yield)(Env *e) { return ESUCCESS; }

/* Go exits on fatal errors (out of memory, for example); unwind to the entry point. */
void WASI(proc_exit)(Env *e, u32 code) { wasm_rt_trap(WASM_RT_TRAP_UNREACHABLE); }
