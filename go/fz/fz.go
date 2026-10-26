// Package fz runs Go's compress/flate and github.com/klauspost/compress/flate on a whole input,
// shared by the c-archive (../main.go) and wasm (../wasm) builds.
package fz

import (
	"bytes"
	stdflate "compress/flate"

	kpflate "github.com/klauspost/compress/flate"
)

// Stateless is the pseudo-level selecting klauspost's stateless encoder.
const Stateless = -100

// Std compresses src with compress/flate in one Write. Errors only on invalid levels.
func Std(src []byte, level int) ([]byte, error) {
	var buf bytes.Buffer
	w, err := stdflate.NewWriter(&buf, level)
	if err != nil {
		return nil, err
	}
	if _, err := w.Write(src); err != nil {
		return nil, err
	}
	if err := w.Close(); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}

// Klauspost compresses src with klauspost/compress/flate in one Write, or with
// StatelessDeflate for the Stateless level. Errors only on invalid levels.
func Klauspost(src []byte, level int) ([]byte, error) {
	var buf bytes.Buffer
	if level == Stateless {
		if err := kpflate.StatelessDeflate(&buf, src, true, nil); err != nil {
			return nil, err
		}
		return buf.Bytes(), nil
	}
	w, err := kpflate.NewWriter(&buf, level)
	if err != nil {
		return nil, err
	}
	if _, err := w.Write(src); err != nil {
		return nil, err
	}
	if err := w.Close(); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}
