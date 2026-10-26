/* The WASI host of the `go` unit, shared by wasi.c and the entry points in ../go.c. */
#ifndef FZ_GO_WASI_H
#define FZ_GO_WASI_H

#include "fz_go.h"

/* wasm2c passes this to every import; one per instance. */
struct w2c_wasi__snapshot__preview1 {
    w2c_fz__go *inst;
    u64 clock; /* fake nanoseconds, see clock_time_get */
};

#endif
