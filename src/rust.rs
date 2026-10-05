//! Pure Rust encoders from crates.io.

use crate::backends::{STRATEGIES, ZLIB_PARAMS};
use crate::{Backend, Error, Param, Result};
use std::io::Write;

/// A backend defined by a plain function.
pub struct RustBackend {
    name: &'static str,
    about: &'static str,
    params: &'static [Param],
    compress: fn(&[u8], &[i64]) -> Result<Vec<u8>>,
}

impl Backend for RustBackend {
    fn name(&self) -> &'static str {
        self.name
    }
    fn about(&self) -> &'static str {
        self.about
    }
    fn params(&self) -> &'static [Param] {
        self.params
    }
    fn compress(&self, input: &[u8], values: &[i64]) -> Result<Vec<u8>> {
        (self.compress)(input, values)
    }
}

fn io_err(name: &str) -> impl Fn(std::io::Error) -> Error + '_ {
    move |e| Error::Encode(format!("{name}: {e}"))
}

pub static ZLIB_RS: RustBackend = RustBackend {
    name: "zlib-rs",
    about: "zlib-rs 0.6.8 (Rust port of zlib-ng by trifectatech)",
    params: ZLIB_PARAMS,
    compress: |input, v| {
        use zlib_rs::{DeflateConfig, ReturnCode, Strategy};
        let strategy = match v[3] {
            0 => Strategy::Default,
            1 => Strategy::Filtered,
            2 => Strategy::HuffmanOnly,
            3 => Strategy::Rle,
            _ => Strategy::Fixed,
        };
        let config = DeflateConfig {
            level: v[0] as i32,
            method: Default::default(),
            window_bits: -(v[1] as i32),
            mem_level: v[2] as i32,
            strategy,
        };
        let mut out = vec![0u8; zlib_rs::compress_bound(input.len())];
        let (written, rc) = zlib_rs::compress_slice(&mut out, input, config);
        if rc != ReturnCode::Ok {
            return Err(Error::Encode(format!("zlib-rs: {rc:?}")));
        }
        let n = written.len();
        out.truncate(n);
        Ok(out)
    },
};

pub static MINIZ_OXIDE: RustBackend = RustBackend {
    name: "miniz-oxide",
    about: "miniz_oxide 0.9.1 (Rust port of miniz)",
    params: &[
        Param::new("level", 6, 0, 10, "compression level (10 = uber)"),
        Param::new("strategy", 0, 0, 4, "zlib-style strategy").names(STRATEGIES),
    ],
    compress: |mut input, v| {
        use miniz_oxide::deflate::core::{
            CompressorOxide, TDEFLFlush, TDEFLStatus, compress, create_comp_flags_from_zip_params,
        };
        let mut c = CompressorOxide::new(create_comp_flags_from_zip_params(
            v[0] as i32,
            -15,
            v[1] as i32,
        ));
        let mut out = vec![0u8; input.len() / 2 + 64];
        let mut pos = 0;
        loop {
            let (status, consumed, written) =
                compress(&mut c, input, &mut out[pos..], TDEFLFlush::Finish);
            pos += written;
            input = &input[consumed..];
            match status {
                TDEFLStatus::Done => break,
                TDEFLStatus::Okay => out.resize(out.len() * 2, 0),
                s => return Err(Error::Encode(format!("miniz-oxide: {s:?}"))),
            }
        }
        out.truncate(pos);
        Ok(out)
    },
};

pub static YAZI: RustBackend = RustBackend {
    name: "yazi",
    about: "yazi 0.2.1 (Rust, derived from miniz)",
    params: &[Param::new("level", 6, 0, 10, "compression level")],
    compress: |input, v| {
        yazi::compress(
            input,
            yazi::Format::Raw,
            yazi::CompressionLevel::Specific(v[0] as u8),
        )
        .map_err(|e| Error::Encode(format!("yazi: {e:?}")))
    },
};

pub static ZOPFLI_RS: RustBackend = RustBackend {
    name: "zopfli-rs",
    about: "zopfli 0.8.3 (Rust port of Zopfli)",
    params: &[
        Param::new("iter", 15, 1, i64::MAX, "number of iterations")
            .sweep(&[1, 2, 5, 10, 15, 50, 100]),
        Param::new(
            "splitmax",
            15,
            0,
            u16::MAX as i64,
            "maximum number of block splits (0 = unlimited)",
        )
        .sweep(&[0, 1, 5, 15, 30]),
    ],
    compress: |input, v| {
        let options = zopfli_rs::Options {
            iteration_count: std::num::NonZeroU64::new(v[0] as u64).unwrap(),
            iterations_without_improvement: std::num::NonZeroU64::MAX,
            maximum_block_splits: v[1] as u16,
        };
        let mut out = Vec::new();
        zopfli_rs::compress(options, zopfli_rs::Format::Deflate, input, &mut out)
            .map_err(|e| Error::Encode(format!("zopfli-rs: {e}")))?;
        Ok(out)
    },
};

pub static DEFLATE_RS: RustBackend = RustBackend {
    name: "deflate-rs",
    about: "deflate 1.0.0 (Rust, oyvindln); presets: fast = 1,0,greedy; default = 128,32,lazy; high = 1768,128,lazy",
    params: &[
        Param::new(
            "checks",
            128,
            0,
            u16::MAX as i64,
            "max_hash_checks (0 with lazy = RLE only)",
        )
        .sweep(&[0, 1, 4, 16, 32, 128, 512, 1768]),
        Param::new("lazy_below", 32, 0, 258, "lazy_if_less_than")
            .sweep(&[0, 8, 16, 32, 64, 128, 258]),
        Param::new("matching", 1, 0, 1, "matching type").names(&[("greedy", 0), ("lazy", 1)]),
    ],
    compress: |input, v| {
        use deflate_rs::{CompressionOptions, MatchingType};
        let o = CompressionOptions {
            max_hash_checks: v[0] as u16,
            lazy_if_less_than: v[1] as u16,
            matching_type: if v[2] == 0 {
                MatchingType::Greedy
            } else {
                MatchingType::Lazy
            },
            ..Default::default()
        };
        Ok(deflate_rs::deflate_bytes_conf(input, o))
    },
};

pub static LIBFLATE: RustBackend = RustBackend {
    name: "libflate",
    about: "libflate 2.3.2 (Rust, sile)",
    params: &[
        Param::new("mode", 0, 0, 2, "block type").names(&[
            ("dynamic", 0),
            ("fixed", 1),
            ("stored", 2),
        ]),
        Param::new("window", 32768, 1, 32768, "LZ77 window size")
            .sweep(&[1024, 4096, 8192, 16384, 32768]),
        Param::new("block", 1 << 20, 1, 1 << 30, "block size").sweep(&[16384, 65536, 1 << 20]),
    ],
    compress: |input, v| {
        use libflate::deflate::{EncodeOptions, Encoder};
        use libflate::lz77::DefaultLz77Encoder;
        let mut o = EncodeOptions::with_lz77(DefaultLz77Encoder::with_window_size(v[1] as u16))
            .block_size(v[2] as usize);
        match v[0] {
            1 => o = o.fixed_huffman_codes(),
            2 => o = o.no_compression(),
            _ => {}
        }
        let mut enc = Encoder::with_options(Vec::new(), o);
        enc.write_all(input).map_err(io_err("libflate"))?;
        enc.finish().into_result().map_err(io_err("libflate"))
    },
};

pub static FDEFLATE: RustBackend = RustBackend {
    name: "fdeflate",
    about: "fdeflate 0.3.7 (Rust, image-rs; PNG-oriented fast encoder)",
    params: &[Param::new("mode", 0, 0, 1, "encoder").names(&[("fast", 0), ("stored", 1)])],
    compress: |input, v| {
        let z = if v[0] == 0 {
            let mut c = fdeflate::Compressor::new(Vec::new()).map_err(io_err("fdeflate"))?;
            c.write_data(input).map_err(io_err("fdeflate"))?;
            c.finish().map_err(io_err("fdeflate"))?
        } else {
            let mut c = fdeflate::StoredOnlyCompressor::new(std::io::Cursor::new(Vec::new()))
                .map_err(io_err("fdeflate"))?;
            c.write_data(input).map_err(io_err("fdeflate"))?;
            c.finish().map_err(io_err("fdeflate"))?.into_inner()
        };
        strip_zlib(z, "fdeflate")
    },
};

fn strip_zlib(mut z: Vec<u8>, name: &str) -> Result<Vec<u8>> {
    if z.len() < 6 || z[1] & 0x20 != 0 {
        return Err(Error::Encode(format!("{name}: malformed zlib stream")));
    }
    z.truncate(z.len() - 4);
    z.drain(..2);
    Ok(z)
}
