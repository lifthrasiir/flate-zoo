module fzgo

go 1.25

// Pinned because the wasm2c translation (fetch.sh) embeds this exact compress/flate.
toolchain go1.25.5

require github.com/klauspost/compress v1.20.1
