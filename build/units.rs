//! Table of C/C++ units. Paths are relative to the crate root.

use super::{Unit, out_dir};
use std::fs;
use std::path::PathBuf;

/// Copies `src` into OUT_DIR with textual replacements applied (each must match exactly once).
fn patched(unit: &str, src: &str, edits: &[(&str, &str)]) -> PathBuf {
    let mut text = fs::read_to_string(src).unwrap_or_else(|e| panic!("{src}: {e}"));
    for (from, to) in edits {
        assert_eq!(
            text.matches(from).count(),
            1,
            "{src}: patch anchor {from:?} must match exactly once"
        );
        text = text.replace(from, to);
    }
    let dir = out_dir().join("patched").join(unit);
    fs::create_dir_all(&dir).unwrap();
    let dst = dir.join(PathBuf::from(src).file_name().unwrap());
    fs::write(&dst, text).unwrap();
    dst
}

fn zlib_ng_headers() -> PathBuf {
    let dir = out_dir().join("gen").join("zlib-ng");
    fs::create_dir_all(&dir).unwrap();
    let v = "vendor/zlib-ng";
    let h = fs::read_to_string(format!("{v}/zlib-ng.h.in"))
        .unwrap()
        .replace("@ZLIB_SYMBOL_PREFIX@", "");
    fs::write(dir.join("zlib-ng.h"), h).unwrap();
    fs::copy(format!("{v}/zconf-ng.h.in"), dir.join("zconf-ng.h")).unwrap();
    fs::copy(
        format!("{v}/zlib_name_mangling.h.empty"),
        dir.join("zlib_name_mangling-ng.h"),
    )
    .unwrap();
    dir
}

/// Patches JavaScript sources into OUT_DIR/patched, where `src/js.rs` includes them from.
pub fn js() {
    // zlib.js: with lazy matching, a match deferred at the last position that still gets a
    // lookup is emitted, and then the bytes it covers are emitted again as literals. `q` is
    // the skip count, which writing the match (`c`) has just set to its length - 2.
    patched(
        "js-zlibjs",
        "vendor/js-zlibjs/bin/rawdeflate.min.js",
        &[(
            "if(f+3>=a){x&&c(x,-1);b=0;",
            "if(f+3>=a){b=0;x&&(c(x,-1),b=q+1);",
        )],
    );
}

pub fn all() -> Vec<Unit> {
    let zlib_srcs = ["adler32.c", "crc32.c", "deflate.c", "trees.c", "zutil.c"];
    let is_x86 = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "x86_64" || a == "x86");
    let is_aarch64 = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "aarch64");

    // Cloudflare's fill_window() only slides the hash chains with NEON or SSE2 and silently skips
    // them otherwise, corrupting every stream longer than the window. This adds the scalar path.
    let slide_fallback = (
        "            }\n\n#endif\n            more += wsize;",
        "            }\n\n#else\n\n            for (i = 0; i < (int)n; i++)\n                s->head[i] = (Pos)(s->head[i] >= wsize ? s->head[i] - wsize : NIL);\n            for (i = 0; i < (int)wsize; i++)\n                s->prev[i] = (Pos)(s->prev[i] >= wsize ? s->prev[i] - wsize : NIL);\n\n#endif\n            more += wsize;",
    );
    let cloudflare_srcs = ["adler32.c", "crc32.c", "trees.c", "zutil.c"];
    let mut cloudflare = Unit::new("zlib-cloudflare")
        .src("vendor/zlib-cloudflare", &cloudflare_srcs)
        .file(patched(
            "zlib-cloudflare",
            "vendor/zlib-cloudflare/deflate.c",
            &[slide_fallback],
        ))
        .inc("vendor/zlib-cloudflare")
        .shim("csrc/zlib_family.c");
    if is_x86 {
        // Without SSE 4.2 Cloudflare silently falls back to a different hash function, which is
        // what `zlib-cloudflare-generic` is for.
        cloudflare = cloudflare
            .def("HAS_SSE42", None)
            .def("HAS_SSE2", None)
            .flag("-msse4.2");
    }
    // Cloudflare's crc32.c (and deflate.c's hash) use ARMv8 CRC32 intrinsics unconditionally on
    // aarch64, which GCC only accepts when the extension is enabled.
    let aarch64_crc = |u: Unit| {
        if is_aarch64 {
            u.flag("-march=armv8-a+crc")
        } else {
            u
        }
    };
    let cloudflare = aarch64_crc(cloudflare);
    let cloudflare_generic_deflate = patched(
        "zlib-cloudflare-generic",
        "vendor/zlib-cloudflare/deflate.c",
        &[
            (
                "#ifdef __aarch64__\n\n#include <arm_neon.h>\n#include <arm_acle.h>\nstatic uint32_t hash_func",
                "#ifdef __aarch64__\n#include <arm_neon.h>\n#endif\n#if 0\nstatic uint32_t hash_func",
            ),
            slide_fallback,
        ],
    );

    vec![
        Unit::new("zlib")
            .src("vendor/zlib", &zlib_srcs)
            .inc("vendor/zlib")
            .shim("csrc/zlib_family.c"),
        Unit::new("zlib-chromium")
            .src("vendor/zlib-chromium", &zlib_srcs)
            .src("vendor/zlib-chromium", &["cpu_features.c"])
            .inc("vendor/zlib-chromium")
            .shim("csrc/zlib_family.c"),
        Unit::new("zlib-chromium-rk")
            .feature("zlib-chromium")
            .src("vendor/zlib-chromium", &zlib_srcs)
            .src("vendor/zlib-chromium", &["cpu_features.c"])
            .inc("vendor/zlib-chromium")
            .def("CHROMIUM_ZLIB_NO_CASTAGNOLI", None)
            .shim("csrc/zlib_family.c"),
        cloudflare,
        aarch64_crc(
            Unit::new("zlib-cloudflare-generic")
                .feature("zlib-cloudflare")
                .src("vendor/zlib-cloudflare", &cloudflare_srcs)
                .file(cloudflare_generic_deflate)
                .inc("vendor/zlib-cloudflare")
                .shim("csrc/zlib_family.c"),
        ),
        Unit::new("zlib-ng")
            .src(
                "vendor/zlib-ng",
                &[
                    "adler32.c",
                    "crc32.c",
                    "crc32_braid_comb.c",
                    "deflate.c",
                    "deflate_fast.c",
                    "deflate_huff.c",
                    "deflate_medium.c",
                    "deflate_quick.c",
                    "deflate_rle.c",
                    "deflate_slow.c",
                    "deflate_stored.c",
                    "functable.c",
                    "insert_string.c",
                    "insert_string_roll.c",
                    "trees.c",
                    "zutil.c",
                    "cpu_features.c",
                ],
            )
            .src(
                "vendor/zlib-ng/arch/generic",
                &[
                    "adler32_c.c",
                    "adler32_fold_c.c",
                    "chunkset_c.c",
                    "compare256_c.c",
                    "crc32_braid_c.c",
                    "crc32_chorba_c.c",
                    "crc32_fold_c.c",
                    "slide_hash_c.c",
                ],
            )
            .inc(zlib_ng_headers())
            .inc("vendor/zlib-ng")
            .def("WITH_ALL_FALLBACKS", None)
            .def("HAVE_BUILTIN_CTZ", None)
            .def("HAVE_BUILTIN_CTZLL", None)
            .def("HAVE_ATTRIBUTE_ALIGNED", None)
            .shim("csrc/zlib_ng.c"),
        Unit::new("libdeflate")
            .src(
                "vendor/libdeflate/lib",
                &[
                    "deflate_compress.c",
                    "utils.c",
                    "arm/cpu_features.c",
                    "x86/cpu_features.c",
                ],
            )
            .inc("vendor/libdeflate")
            .shim("csrc/libdeflate.c"),
        Unit::new("zopfli")
            .src(
                "vendor/zopfli/src/zopfli",
                &[
                    "blocksplitter.c",
                    "cache.c",
                    "deflate.c",
                    "gzip_container.c",
                    "hash.c",
                    "katajainen.c",
                    "lz77.c",
                    "squeeze.c",
                    "tree.c",
                    "util.c",
                    "zlib_container.c",
                    "zopfli_lib.c",
                ],
            )
            .inc("vendor/zopfli/src")
            .shim("csrc/zopfli.c"),
        Unit::new("miniz")
            .src(
                "vendor/miniz",
                &["miniz.c", "miniz_tdef.c", "miniz_tinfl.c"],
            )
            .inc("csrc/miniz")
            .inc("vendor/miniz")
            .shim("csrc/miniz.c"),
        Unit::new("stb")
            .src("csrc/impl", &["stb.c"])
            .inc("vendor/stb")
            .shim("csrc/stb.c"),
        Unit::new("lodepng")
            .src("csrc/impl", &["lodepng.c"])
            .inc("vendor/lodepng")
            .shim("csrc/lodepng.c"),
        Unit::new("sdefl")
            .src("csrc/impl", &["sdefl.c"])
            .inc("vendor/vurtun-lib")
            .shim("csrc/sdefl.c"),
        Unit::new("uzlib")
            .src("vendor/uzlib/src", &["genlz77.c", "defl_static.c"])
            .inc("vendor/uzlib/src")
            .shim("csrc/uzlib.c"),
        Unit::new("libslz")
            .src("vendor/libslz/src", &["slz.c"])
            .inc("vendor/libslz/src")
            .shim("csrc/libslz.c"),
        Unit::new("gzip")
            .src("vendor/gzip", &["deflate.c", "trees.c", "bits.c"])
            .src("csrc/gzip", &["glue.c"])
            .inc("csrc/gzip")
            .inc("vendor/gzip")
            .shim("csrc/gzip.c"),
        Unit::new("infozip")
            .src("vendor/infozip", &["deflate.c", "trees.c"])
            .src("csrc/infozip", &["glue.c"])
            .inc("vendor/infozip")
            .inc("vendor/infozip/unix")
            .def("UNIX", None)
            .shim("csrc/infozip.c"),
        Unit::new("isa-l")
            .src(
                "vendor/isa-l/igzip",
                &[
                    "igzip.c",
                    "hufftables_c.c",
                    "igzip_base.c",
                    "igzip_icf_base.c",
                    "adler32_base.c",
                    "flatten_ll.c",
                    "encode_df.c",
                    "igzip_icf_body.c",
                    "igzip_base_aliases.c",
                    "proc_heap_base.c",
                    "huff_codes.c",
                    "igzip_inflate.c",
                ],
            )
            .src(
                "vendor/isa-l/crc",
                &["crc_base.c", "crc64_base.c", "crc_base_aliases.c"],
            )
            .inc("vendor/isa-l/include")
            .inc("vendor/isa-l/igzip")
            .inc("vendor/isa-l/crc")
            .shim("csrc/isal.c"),
        Unit::new("sevenzip")
            .feature("7zip")
            .cpp()
            .mixed()
            .src(
                "vendor/7zip/C",
                &["Alloc.c", "CpuArch.c", "HuffEnc.c", "LzFind.c", "Sort.c"],
            )
            .src(
                "vendor/7zip/CPP/7zip",
                &[
                    "Compress/DeflateEncoder.cpp",
                    "Common/CWrappers.cpp",
                    "Common/OutBuffer.cpp",
                    "Common/StreamUtils.cpp",
                ],
            )
            .inc("vendor/7zip/CPP/7zip/Compress")
            .shim("csrc/sevenzip.cpp"),
        Unit::new("advancecomp-7z")
            .cpp()
            .src(
                "vendor/advancecomp/7z",
                &[
                    "CRC.cc",
                    "DeflateEncoder.cc",
                    "HuffmanEncoder.cc",
                    "IInOutStreams.cc",
                    "LSBFEncoder.cc",
                    "OutByte.cc",
                    "WindowIn.cc",
                ],
            )
            .inc("vendor/advancecomp")
            // keep the old stream classes apart from 7-Zip's COM-style ones of the same name
            .def("ISequentialInStream", Some("AdvISequentialInStream"))
            .def("ISequentialOutStream", Some("AdvISequentialOutStream"))
            .shim("csrc/advancecomp7z.cpp"),
        Unit::new("cryptopp")
            .cpp()
            .src(
                "vendor/cryptopp",
                &[
                    "algparam.cpp",
                    "allocate.cpp",
                    "cryptlib.cpp",
                    "filters.cpp",
                    "fips140.cpp",
                    "misc.cpp",
                    "mqueue.cpp",
                    "queue.cpp",
                    "zdeflate.cpp",
                ],
            )
            .inc("vendor/cryptopp")
            .def("CRYPTOPP_DISABLE_ASM", None)
            .shim("csrc/cryptopp.cpp"),
        {
            // Mirrors vendor/ect/src/CMakeLists.txt: -O3, ZLIB_CONST, SIMD/CRC flags for ECT's zlib
            // (only used by level 1) and multithreading support left on (it only changes
            // `thread_local` into a real TLS, so concurrent calls are safe).
            //
            // Zopfli's squeeze.c keeps a match finder and cost model in thread-locals that carry
            // over to the next call on the same thread (the match finder even points into the
            // previous, freed input). The CLI runs one call per process; we add a reset function.
            let squeeze_reset = (
                "static thread_local SymbolStats st;\n",
                "static thread_local SymbolStats st;\n\n\
                 void ZopfliResetThreadState(void) {\n\
                 \x20 if (right) MatchFinder_Free(&mf);\n\
                 \x20 memset(&mf, 0, sizeof(mf));\n\
                 \x20 right = 0;\n\
                 \x20 memset(&st, 0, sizeof(st));\n\
                 }\n",
            );
            let mut ect = Unit::new("ect")
                .mixed()
                .src("vendor/ect/src", &["LzFind.c"])
                .src(
                    "vendor/ect/src/zopfli",
                    &[
                        "blocksplitter.c",
                        "deflate.cpp",
                        "katajainen.cpp",
                        "lz77.c",
                        "util.c",
                    ],
                )
                .file(patched(
                    "ect",
                    "vendor/ect/src/zopfli/squeeze.c",
                    &[squeeze_reset],
                ))
                .src(
                    "vendor/ect/src/zlib",
                    &[
                        "adler32.c",
                        "adler32_simd.c",
                        "crc32.c",
                        "crc32_simd.c",
                        "deflate.c",
                        "trees.c",
                        "zutil.c",
                    ],
                )
                .inc("vendor/ect/src")
                // for the patched squeeze.c's own includes ("util.h", "../LzFind.h", ...)
                .inc("vendor/ect/src/zopfli")
                .inc("vendor/ect/src/zlib")
                .def("ZLIB_CONST", None)
                .flag("-O3")
                .shim("csrc/ect.cpp");
            ect = if is_x86 {
                ect.def("ADLER32_SIMD_SSSE3", None)
                    .def("HAS_PCLMUL", None)
                    .flag("-mpclmul")
                    .flag("-msse4.2")
            } else if is_aarch64 {
                ect.def("ADLER32_SIMD_NEON", None)
                    .flag("-march=armv8-a+crc")
            } else {
                ect
            };
            ect.link(
                if std::env::var("CARGO_CFG_TARGET_VENDOR").is_ok_and(|v| v == "apple") {
                    "c++"
                } else {
                    "stdc++"
                },
            )
        },
    ]
}
