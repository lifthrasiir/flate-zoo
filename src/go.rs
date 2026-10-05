//! Go encoders, linked from the c-archive built out of `go/`.

use crate::Param;
use crate::native::native;

const GO_LEVELS: &[(&str, i64)] = &[("huffman", -2)];
const KLAUSPOST_LEVELS: &[(&str, i64)] = &[("huffman", -2), ("stateless", -100)];

native!(pub GO = fz_go_std_run(
    "go",
    "Go compress/flate (standard library of the Go toolchain used to build; tested with 1.25)",
    &[Param::new("level", 6, 0, 9, "compression level").names(GO_LEVELS).sweep(&[-2, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9])]
));
native!(pub GO_KLAUSPOST = fz_go_klauspost_run(
    "go-klauspost",
    "github.com/klauspost/compress/flate v1.20.1",
    &[Param::new("level", 6, 0, 9, "compression level")
        .names(KLAUSPOST_LEVELS)
        .sweep(&[-100, -2, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9])]
));
