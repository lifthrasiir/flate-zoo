//! Registry of all compiled-in backends and the parameter tables of the native ones.

// `go` enables `native` without using it here.
#[cfg(feature = "native")]
#[allow(unused_imports)]
use crate::native::native;
#[allow(unused_imports)]
use crate::{Backend, Param};

#[cfg(any(
    feature = "zlib",
    feature = "zlib-ng",
    feature = "zlib-chromium",
    feature = "zlib-cloudflare",
    feature = "miniz",
    feature = "rust",
    feature = "js"
))]
pub(crate) const STRATEGIES: &[(&str, i64)] = &[
    ("default", 0),
    ("filtered", 1),
    ("huffman", 2),
    ("rle", 3),
    ("fixed", 4),
];

/// Parameters of `deflateInit2` in zlib and every API-compatible implementation.
#[cfg(any(
    feature = "zlib",
    feature = "zlib-ng",
    feature = "zlib-chromium",
    feature = "zlib-cloudflare",
    feature = "rust",
    feature = "js"
))]
pub(crate) const ZLIB_PARAMS: &[Param] = &[
    Param::new("level", 6, 0, 9, "compression level"),
    Param::new("wbits", 15, 9, 15, "log2 of the window size"),
    Param::new("mem", 8, 1, 9, "memLevel (hash table size is 2^(mem+7))"),
    Param::new("strategy", 0, 0, 4, "deflate strategy").names(STRATEGIES),
];

/// [`ZLIB_PARAMS`] plus the buffer sizes of each `deflate()` call (see csrc/zlib_family.c).
/// They only matter for stored blocks (level 0), which zlib sizes after avail_in/avail_out.
#[cfg(any(
    feature = "zlib",
    feature = "zlib-ng",
    feature = "zlib-chromium",
    feature = "zlib-cloudflare"
))]
pub(crate) const ZLIB_STREAM_PARAMS: &[Param] = &[
    ZLIB_PARAMS[0],
    ZLIB_PARAMS[1],
    ZLIB_PARAMS[2],
    ZLIB_PARAMS[3],
    Param::new(
        "inbuf",
        0,
        0,
        1 << 30,
        "avail_in per deflate() call (0 = all at once)",
    )
    .sweep(&[0, 4096, 16384, 65536]),
    Param::new(
        "outbuf",
        0,
        0,
        1 << 30,
        "avail_out per deflate() call (0 = enough)",
    )
    .sweep(&[0, 4096, 16384, 65536]),
];

#[cfg(feature = "zlib")]
native!(ZLIB = fz_zlib_run("zlib", "zlib 1.3.1 (madler)", ZLIB_STREAM_PARAMS));
#[cfg(feature = "zlib-ng")]
native!(
    ZLIB_NG = fz_zlib_ng_run(
        "zlib-ng",
        "zlib-ng 2.3.3, native API, generic (non-SIMD) build",
        ZLIB_STREAM_PARAMS
    )
);
#[cfg(feature = "zlib-chromium")]
native!(
    ZLIB_CHROMIUM = fz_zlib_chromium_run(
        "zlib-chromium",
        "Chromium's zlib fork (third_party/zlib @ 456ae73), default multiplicative hash",
        ZLIB_STREAM_PARAMS
    )
);
#[cfg(feature = "zlib-chromium")]
native!(
    ZLIB_CHROMIUM_RK = fz_zlib_chromium_rk_run(
        "zlib-chromium-rk",
        "Chromium's zlib fork built with CHROMIUM_ZLIB_NO_CASTAGNOLI (Rabin-Karp rolling hash)",
        ZLIB_STREAM_PARAMS
    )
);
#[cfg(feature = "zlib-cloudflare")]
native!(
    ZLIB_CLOUDFLARE = fz_zlib_cloudflare_run(
        "zlib-cloudflare",
        "Cloudflare's zlib fork (@ 3944b7d), CRC32C hash as built on aarch64 or x86 with SSE 4.2",
        ZLIB_STREAM_PARAMS
    )
);
#[cfg(feature = "zlib-cloudflare")]
native!(
    ZLIB_CLOUDFLARE_GENERIC = fz_zlib_cloudflare_generic_run(
        "zlib-cloudflare-generic",
        "Cloudflare's zlib fork (@ 3944b7d), portable multiply-xor hash as built without SSE 4.2",
        ZLIB_STREAM_PARAMS
    )
);

#[cfg(feature = "libdeflate")]
native!(
    LIBDEFLATE = fz_libdeflate_run(
        "libdeflate",
        "libdeflate 1.26 (ebiggers)",
        &[Param::new("level", 6, 0, 12, "compression level")]
    )
);

#[cfg(feature = "zopfli")]
native!(
    ZOPFLI = fz_zopfli_run(
        "zopfli",
        "Zopfli (google/zopfli @ df1517d), C reference implementation",
        &[
            Param::new("iter", 15, 1, i32::MAX as i64, "number of iterations")
                .sweep(&[1, 2, 5, 10, 15, 50, 100]),
            Param::new("split", 1, 0, 1, "block splitting"),
            Param::new(
                "splitmax",
                15,
                0,
                i32::MAX as i64,
                "maximum number of blocks (0 = unlimited)"
            )
            .sweep(&[0, 1, 5, 15, 30]),
        ]
    )
);

#[cfg(feature = "miniz")]
native!(
    MINIZ = fz_miniz_run(
        "miniz",
        "miniz 3.1.2 (richgel999) tdefl, flags from tdefl_create_comp_flags_from_zip_params",
        &[
            Param::new("level", 6, 0, 10, "compression level (10 = uber)"),
            Param::new("strategy", 0, 0, 4, "zlib-style strategy").names(STRATEGIES),
        ]
    )
);

#[cfg(feature = "stb")]
native!(
    STB = fz_stb_run(
        "stb",
        "stb_image_write.h stbi_zlib_compress (nothings/stb @ 2c980bb), always one fixed-Huffman block",
        &[Param::new(
            "quality",
            8,
            5,
            1 << 20,
            "hash bucket size; stb_image_write's PNG default is 8"
        )
        .sweep(&[5, 6, 7, 8, 9, 10, 12, 16, 32, 64])]
    )
);

#[cfg(feature = "lodepng")]
native!(
    LODEPNG = fz_lodepng_run(
        "lodepng",
        "LodePNG lodepng_deflate (lvandeve/lodepng @ ff206aa)",
        &[
            Param::new("btype", 2, 0, 2, "block type").names(&[
                ("stored", 0),
                ("fixed", 1),
                ("dynamic", 2)
            ]),
            Param::new("lz77", 1, 0, 1, "use LZ77"),
            Param::new("window", 2048, 1, 32768, "window size (power of two)")
                .sweep(&[256, 512, 1024, 2048, 4096, 8192, 16384, 32768]),
            Param::new("minmatch", 3, 3, 258, "minimum match length").sweep(&[3, 4, 5, 6]),
            Param::new("nicematch", 128, 3, 258, "stop searching at this length")
                .sweep(&[16, 32, 64, 128, 258]),
            Param::new("lazy", 1, 0, 1, "lazy matching"),
        ]
    )
);

#[cfg(feature = "sdefl")]
native!(
    SDEFL = fz_sdefl_run(
        "sdefl",
        "sdefl.h (vurtun/lib @ 5a3f3ab)",
        &[Param::new("level", 5, 0, 8, "compression level")]
    )
);

#[cfg(feature = "uzlib")]
native!(
    UZLIB = fz_uzlib_run(
        "uzlib",
        "uzlib (pfalcon/uzlib @ 6d60d65) compressor, derived from PuTTY's; fixed Huffman only",
        &[
            Param::new("hash_bits", 12, 1, 24, "log2 of hash table size")
                .sweep(&[8, 10, 12, 14, 16]),
            Param::new("dict", 32768, 1, 32768, "maximum match distance")
                .sweep(&[256, 1024, 4096, 8192, 16384, 32768]),
        ]
    )
);

#[cfg(feature = "libslz")]
native!(
    LIBSLZ = fz_libslz_run(
        "libslz",
        "libslz (wtarreau/libslz @ daf74c4), stateless zlib-compatible encoder",
        &[Param::new(
            "level",
            1,
            0,
            1,
            "0 = stored only, 1 = compressed"
        )]
    )
);

#[cfg(all(feature = "apple", target_vendor = "apple"))]
mod apple {
    use crate::{Backend, Error, Param, Result};

    unsafe extern "C" {
        fn compression_encode_buffer(
            dst: *mut u8,
            dst_size: usize,
            src: *const u8,
            src_size: usize,
            scratch: *mut std::ffi::c_void,
            algorithm: u32,
        ) -> usize;
    }
    const COMPRESSION_ZLIB: u32 = 0x205;

    pub struct Apple;

    #[link(name = "compression")]
    unsafe extern "C" {}

    impl Backend for Apple {
        fn name(&self) -> &'static str {
            "apple"
        }
        fn about(&self) -> &'static str {
            "Apple libcompression COMPRESSION_ZLIB (raw DEFLATE, level 5), OS-provided"
        }
        fn params(&self) -> &'static [Param] {
            &[]
        }
        fn compress(&self, input: &[u8], _: &[i64]) -> Result<Vec<u8>> {
            if input.is_empty() {
                // compression_encode_buffer reports failure as 0 bytes, so this is ambiguous;
                // an empty final fixed block is what every other encoder emits anyway.
                return Err(Error::Encode("apple: empty input is not supported".into()));
            }
            let mut cap = input.len() + input.len() / 8 + 1024;
            for _ in 0..4 {
                let mut out = vec![0u8; cap];
                // SAFETY: both buffers are valid for the given sizes; scratch may be null.
                let n = unsafe {
                    compression_encode_buffer(
                        out.as_mut_ptr(),
                        cap,
                        input.as_ptr(),
                        input.len(),
                        std::ptr::null_mut(),
                        COMPRESSION_ZLIB,
                    )
                };
                if n != 0 {
                    out.truncate(n);
                    return Ok(out);
                }
                cap *= 2;
            }
            Err(Error::Encode(
                "apple: compression_encode_buffer failed".into(),
            ))
        }
    }
}

#[cfg(any(feature = "gzip", feature = "infozip"))]
static GZIP_INFOZIP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(feature = "gzip")]
native!(
    GZIP = fz_gzip_run(
        "gzip",
        "GNU gzip 1.14 deflate (GPL-3), as written by `gzip -N`",
        &[
            Param::new("level", 6, 1, 9, "compression level"),
            Param::new("rsync", 0, 0, 1, "--rsyncable chunking"),
        ],
        lock = &GZIP_INFOZIP_LOCK
    )
);
#[cfg(feature = "infozip")]
native!(
    INFOZIP = fz_infozip_run(
        "infozip",
        "Info-ZIP Zip 3.0 deflate, as written by `zip -N`",
        &[Param::new("level", 6, 1, 9, "compression level")],
        lock = &GZIP_INFOZIP_LOCK
    )
);
#[cfg(feature = "isa-l")]
native!(
    ISAL = fz_isa_l_run(
        "isa-l",
        "Intel ISA-L igzip (v2.32.1), portable C base functions, not the asm/NEON paths (upstream does not \
         document that their streams are bit-identical; the asm may differ), raw deflate; \
         defaults match the igzip CLI (stateful, 1 MiB input chunks)",
        &[
            Param::new(
                "level",
                1,
                0,
                3,
                "igzip level (level buffer = ISAL_DEF_LVLn_DEFAULT)"
            )
            .sweep(&[0, 1, 2, 3]),
            Param::new(
                "chunk_kib",
                1024,
                1,
                1048576,
                "KiB of input per isal_deflate call (CLI: 1024)"
            )
            .sweep(&[16, 64, 1024]),
            Param::new(
                "stateless",
                0,
                0,
                1,
                "1 = single isal_deflate_stateless call"
            ),
        ]
    )
);

#[cfg(feature = "7zip")]
native!(
    SEVENZIP = fz_sevenzip_run(
        "7zip",
        "7-Zip 26.03 Deflate encoder (CPP/7zip/Compress/DeflateEncoder.cpp, single-threaded); \
         `level` alone reproduces `7z a -tzip|-tgzip -mx=<level>`; the other parameters are \
         the -mpass/-mfb/-mmc/-ma method properties (0 or -1 = derived from level)",
        &[
            Param::new("level", 5, 0, 9, "-mx level (default 5; 1..4 fast algo, 5..6 normal, 7..8 3 passes/fb 64, 9 10 passes/fb 128)")
                .sweep(&[1, 3, 5, 7, 9]),
            Param::new("passes", 0, 0, 255, "-mpass number of passes (0 = from level: 1, 3 or 10)")
                .sweep(&[0, 1, 5, 15]),
            Param::new("fb", 0, 0, 258, "-mfb fast bytes, clamped to 3..258 (0 = from level: 32, 64 or 128)")
                .sweep(&[0, 16, 32, 128, 258]),
            Param::new("mc", 0, 0, 1000000, "-mmc match finder cycles (0 = 16 + fb/2)")
                .sweep(&[0, 4, 64, 1000]),
            Param::new("algo", -1, -1, 1, "-ma: 0 = fast (hash chains), 1 = normal (binary tree); -1 = from level")
                .sweep(&[-1, 0, 1]),
        ]
    )
);
#[cfg(feature = "advancecomp-7z")]
native!(
    ADVANCECOMP_7Z = fz_advancecomp_7z_run(
        "advancecomp-7z",
        "AdvanceCOMP 2.6 embedded legacy 7-Zip deflate encoder (compress_deflate_7z); advdef -4 / \
         advzip -4 use passes = max(15, -i N), fb = 255",
        &[
            Param::new(
                "passes",
                15,
                1,
                255,
                "number of passes (advdef -4: 15, or -i N if larger)"
            )
            .sweep(&[1, 5, 15, 50]),
            Param::new("fb", 255, 3, 255, "fast bytes (advdef -4: 255)")
                .sweep(&[16, 32, 64, 128, 255]),
        ]
    )
);
#[cfg(feature = "cryptopp")]
native!(
    CRYPTOPP = fz_cryptopp_run(
        "cryptopp",
        "Crypto++ 8.9.0 Deflator (zdeflate.cpp)",
        &[
            Param::new("level", 6, 0, 9, "deflateLevel 0..9 (default 6)"),
            Param::new("wbits", 15, 9, 15, "log2WindowSize"),
            Param::new("detect", 1, 0, 1, "detectUncompressible (default on)"),
        ]
    )
);
#[cfg(feature = "ect")]
native!(
    ECT = fz_ect_run(
        "ect",
        "Efficient Compression Tool (fhanau/Efficient-Compression-Tool @ e711c5e) gzip/zip deflate, single-threaded; level 1 is zlib -9",
        &[
            Param::new("level", 3, 1, 9, "ECT mode -1..-9 (1 = zlib level 9, 2..9 = increasingly thorough Zopfli variants)"),
            Param::new("iterations", 0, 0, 9999, "ECT's -NNNN form: iteration count 10..9999 on top of the level 9 settings (overrides level; 0 = off)")
                .sweep(&[0, 10, 20, 50]),
            Param::new("twice", 0, 0, 9, "ECT's -NNNNN form: leading digit, block-split twice when non-zero")
                .sweep(&[0, 1]),
        ]
    )
);

/// All compiled-in backends in a stable order.
pub fn backends() -> &'static [&'static dyn Backend] {
    BACKENDS
}

static BACKENDS: &[&dyn Backend] = &[
    #[cfg(feature = "zlib")]
    &ZLIB,
    #[cfg(feature = "zlib-ng")]
    &ZLIB_NG,
    #[cfg(feature = "zlib-chromium")]
    &ZLIB_CHROMIUM,
    #[cfg(feature = "zlib-chromium")]
    &ZLIB_CHROMIUM_RK,
    #[cfg(feature = "zlib-cloudflare")]
    &ZLIB_CLOUDFLARE,
    #[cfg(feature = "zlib-cloudflare")]
    &ZLIB_CLOUDFLARE_GENERIC,
    #[cfg(feature = "rust")]
    &crate::rust::ZLIB_RS,
    #[cfg(feature = "libdeflate")]
    &LIBDEFLATE,
    #[cfg(feature = "zopfli")]
    &ZOPFLI,
    #[cfg(feature = "rust")]
    &crate::rust::ZOPFLI_RS,
    #[cfg(feature = "miniz")]
    &MINIZ,
    #[cfg(feature = "rust")]
    &crate::rust::MINIZ_OXIDE,
    #[cfg(feature = "rust")]
    &crate::rust::YAZI,
    #[cfg(feature = "rust")]
    &crate::rust::DEFLATE_RS,
    #[cfg(feature = "rust")]
    &crate::rust::LIBFLATE,
    #[cfg(feature = "rust")]
    &crate::rust::FDEFLATE,
    #[cfg(feature = "stb")]
    &STB,
    #[cfg(feature = "lodepng")]
    &LODEPNG,
    #[cfg(feature = "sdefl")]
    &SDEFL,
    #[cfg(feature = "uzlib")]
    &UZLIB,
    #[cfg(feature = "libslz")]
    &LIBSLZ,
    #[cfg(feature = "go")]
    &crate::go::GO,
    #[cfg(feature = "go")]
    &crate::go::GO_KLAUSPOST,
    #[cfg(feature = "gzip")]
    &GZIP,
    #[cfg(feature = "infozip")]
    &INFOZIP,
    #[cfg(feature = "isa-l")]
    &ISAL,
    #[cfg(feature = "7zip")]
    &SEVENZIP,
    #[cfg(feature = "advancecomp-7z")]
    &ADVANCECOMP_7Z,
    #[cfg(feature = "cryptopp")]
    &CRYPTOPP,
    #[cfg(feature = "js")]
    &crate::js::FFLATE,
    #[cfg(feature = "js")]
    &crate::js::PAKO,
    #[cfg(feature = "js")]
    &crate::js::UZIP,
    #[cfg(feature = "js")]
    &crate::js::ZLIBJS,
    #[cfg(feature = "ect")]
    &ECT,
    #[cfg(all(feature = "apple", target_vendor = "apple"))]
    &apple::Apple,
];
