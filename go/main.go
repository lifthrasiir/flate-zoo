// Exposes Go's compress/flate and github.com/klauspost/compress/flate as C entry points with
// the same contract as csrc/fz.h: raw DEFLATE output in a malloc'ed buffer owned by the caller.
//
// This c-archive needs a Go toolchain at build time, so it only backs the `go-native` feature,
// the reference that the wasm2c translation of ./wasm (feature `go`) is tested against.
package main

/*
#include <stdlib.h>
#include <string.h>
*/
import "C"

import (
	"unsafe"

	"fzgo/fz"
)

func run(in *C.uchar, inLen C.size_t, params *C.longlong, nparams C.size_t, out **C.uchar, outLen *C.size_t, compress func([]byte, int) ([]byte, error)) C.int {
	if nparams != 1 {
		return -1
	}
	src := unsafe.Slice((*byte)(unsafe.Pointer(in)), int(inLen))
	res, err := compress(src, int(*params))
	if err != nil {
		return -1 // FZ_EPARAM: both packages only fail on invalid levels
	}
	n := len(res)
	p := C.malloc(C.size_t(n + 1))
	if p == nil {
		return -2
	}
	C.memcpy(p, unsafe.Pointer(unsafe.SliceData(res)), C.size_t(n))
	*out = (*C.uchar)(p)
	*outLen = C.size_t(n)
	return 0
}

//export fz_go_native_std_run
func fz_go_native_std_run(in *C.uchar, inLen C.size_t, params *C.longlong, nparams C.size_t, out **C.uchar, outLen *C.size_t) C.int {
	return run(in, inLen, params, nparams, out, outLen, fz.Std)
}

//export fz_go_native_klauspost_run
func fz_go_native_klauspost_run(in *C.uchar, inLen C.size_t, params *C.longlong, nparams C.size_t, out **C.uchar, outLen *C.size_t) C.int {
	return run(in, inLen, params, nparams, out, outLen, fz.Klauspost)
}

func main() {}
