// A WASI reactor exposing ../fz, built by fetch.sh with GOOS=wasip1 -buildmode=c-shared and
// translated to C with wasm2c. csrc/go.c drives it with one fresh instance per call:
// fz_alloc an input buffer, fill it, fz_run, then read fz_out_len bytes at the returned
// pointer. Nothing is ever freed; the whole instance is discarded instead.
package main

import (
	"unsafe"

	"fzgo/fz"
)

// Keeps buffers handed out to the host reachable.
var pinned [][]byte

var outLen int32

//go:wasmexport fz_alloc
func fzAlloc(n int32) unsafe.Pointer {
	b := make([]byte, n+1)
	pinned = append(pinned, b)
	return unsafe.Pointer(unsafe.SliceData(b))
}

//go:wasmexport fz_out_len
func fzOutLen() int32 { return outLen }

// fz_run compresses n bytes at in with compress/flate (lib 0) or klauspost (lib 1), returning
// the output or nil if the level is invalid.
//
//go:wasmexport fz_run
func fzRun(lib int32, in unsafe.Pointer, n int32, level int32) unsafe.Pointer {
	src := unsafe.Slice((*byte)(in), n)
	compress := fz.Std
	if lib == 1 {
		compress = fz.Klauspost
	}
	res, err := compress(src, int(level))
	if err != nil {
		return nil
	}
	// fz_alloc's buffers are never empty, so the pointer is non-nil even for empty output.
	p := fzAlloc(int32(len(res)))
	copy(unsafe.Slice((*byte)(p), len(res)), res)
	outLen = int32(len(res))
	return p
}

func main() {}
