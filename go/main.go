// Exposes Go's compress/flate and github.com/klauspost/compress/flate as C entry points with
// the same contract as csrc/fz.h: raw DEFLATE output in a malloc'ed buffer owned by the caller.
package main

/*
#include <stdlib.h>
#include <string.h>
*/
import "C"

import (
	"bytes"
	stdflate "compress/flate"
	"io"
	"unsafe"

	kpflate "github.com/klauspost/compress/flate"
)

func run(in *C.uchar, inLen C.size_t, out **C.uchar, outLen *C.size_t, compress func([]byte, io.Writer) error) C.int {
	src := unsafe.Slice((*byte)(unsafe.Pointer(in)), int(inLen))
	var buf bytes.Buffer
	if err := compress(src, &buf); err != nil {
		return -1 // FZ_EPARAM: both packages only fail on invalid levels
	}
	n := buf.Len()
	p := C.malloc(C.size_t(n + 1))
	if p == nil {
		return -2
	}
	C.memcpy(p, unsafe.Pointer(unsafe.SliceData(buf.Bytes())), C.size_t(n))
	*out = (*C.uchar)(p)
	*outLen = C.size_t(n)
	return 0
}

//export fz_go_std_run
func fz_go_std_run(in *C.uchar, inLen C.size_t, params *C.longlong, nparams C.size_t, out **C.uchar, outLen *C.size_t) C.int {
	if nparams != 1 {
		return -1
	}
	level := int(*params)
	return run(in, inLen, out, outLen, func(src []byte, w io.Writer) error {
		fw, err := stdflate.NewWriter(w, level)
		if err != nil {
			return err
		}
		if _, err := fw.Write(src); err != nil {
			return err
		}
		return fw.Close()
	})
}

//export fz_go_klauspost_run
func fz_go_klauspost_run(in *C.uchar, inLen C.size_t, params *C.longlong, nparams C.size_t, out **C.uchar, outLen *C.size_t) C.int {
	if nparams != 1 {
		return -1
	}
	level := int(*params)
	return run(in, inLen, out, outLen, func(src []byte, w io.Writer) error {
		if level == stateless {
			return kpflate.StatelessDeflate(w, src, true, nil)
		}
		fw, err := kpflate.NewWriter(w, level)
		if err != nil {
			return err
		}
		if _, err := fw.Write(src); err != nil {
			return err
		}
		return fw.Close()
	})
}

// Pseudo-level selecting klauspost's stateless encoder.
const stateless = -100

func main() {}
