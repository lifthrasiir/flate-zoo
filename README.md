# 🦒🐘🐒 flate-zoo: a zoo of DEFLATE encoders

This crate puts every DEFLATE encoder I could find behind one Rust interface. It is meant as a
sample generator for DEFLATE recompression research: each backend returns exactly the raw
DEFLATE stream (RFC 1951, with any zlib or gzip framing stripped) that the real
library or tool would produce.

```rust
let enc: flate_zoo::Encoder = "zlib:level=9,strategy=filtered".parse()?;
let raw = enc.compress(&data)?;
assert_eq!(flate_zoo::inflate(&raw)?, data);

for enc in flate_zoo::presets(flate_zoo::Sweep::OneAtATime) { /* ... */ }
```

```
flate-zoo list                                  # backends and their parameters
flate-zoo compress SPEC [IN [OUT]]              # e.g. flate-zoo compress libdeflate:level=12 a.txt a.raw
flate-zoo survey [-s default|one|full] [-b BACKEND]... FILE
                                            # run every preset, verify round trips, group identical outputs
flate-zoo identify [-s ...] [-b ...] RAW_FILE   # find presets that reproduce a raw DEFLATE stream bit-exactly
```

A spec is `backend[:param=value,...]`. Omitted parameters take their defaults, and
`Encoder::to_string()` prints every parameter. Each parameter carries its range and a sweep list,
so `Sweep::OneAtATime` and `Sweep::Full` can enumerate configurations generically.

## Backends

| backend | implementation | parameters |
|---|---|---|
| `zlib` | zlib 1.3.1 | level, wbits, mem, strategy, inbuf, outbuf |
| `zlib-ng` | zlib-ng 2.3.3, native API, generic build | same |
| `zlib-chromium`, `zlib-chromium-rk` | Chromium's fork, with its default hash and with `CHROMIUM_ZLIB_NO_CASTAGNOLI` | same |
| `zlib-cloudflare`, `zlib-cloudflare-generic` | Cloudflare's fork, with the CRC32C hash (aarch64 / SSE4.2) and the portable hash | same |
| `zlib-rs` | zlib-rs 0.6.8 (Rust) | level, wbits, mem, strategy |
| `libdeflate` | libdeflate 1.26 | level 0–12 |
| `zopfli`, `zopfli-rs` | Zopfli in C, and its Rust port | iterations, block splitting |
| `miniz`, `miniz-oxide`, `yazi` | miniz 3.1.2 (C), miniz_oxide 0.9.1 and yazi 0.2.1 (Rust) | level 0–10 (+ strategy) |
| `deflate-rs`, `libflate`, `fdeflate` | Rust crates | crate-specific |
| `stb`, `lodepng`, `sdefl`, `uzlib`, `libslz` | single-file C libraries | library-specific |
| `go`, `go-klauspost` | Go `compress/flate` and klauspost/compress 1.20.1 (Go c-archive) | level (+ huffman, stateless) |
| `gzip`, `infozip` | GNU gzip 1.14 and Info-ZIP Zip 3.0 deflate, driven through in-memory I/O glue | level 1–9 (+ gzip `rsync`) |
| `isa-l` | Intel ISA-L 2.32.1 igzip, portable C base functions | level 0–3, chunk_kib, stateless |
| `7zip` | 7-Zip 26.03 DeflateEncoder | level, passes, fb, mc, algo |
| `advancecomp-7z` | the legacy 7-Zip encoder in AdvanceCOMP 2.6 (`advdef -4`) | passes, fb |
| `cryptopp` | Crypto++ 8.9.0 `Deflator` | level, wbits, detect |
| `ect` | Efficient Compression Tool deflate | level 1–9, iterations, twice |
| `fflate`, `pako`, `uzip`, `zlibjs` | JavaScript libraries running on an embedded QuickJS (rquickjs) | library-specific |
| `apple` | macOS libcompression `COMPRESSION_ZLIB` (macOS only) | — |

Run `flate-zoo list` for the exact parameter ranges and defaults.

## Building

Requirements: a C/C++ compiler, `nm`, Go (for the `go` feature) and network access for
the first fetch.

```
git clone https://github.com/lifthrasiir/flate-zoo.git    # no cargo, sorry
cd flate-zoo
./fetch.sh          # pinned upstream sources into vendor/, plus `go mod vendor` in go/
cargo build --release
cargo test --release
```

Every backend is a cargo feature and all of them are on by default. C units are compiled twice:
the first pass collects their global symbols with `nm`, and the second force-includes a header
that renames them to `fz_<unit>_<symbol>`. That is how four zlib forks, miniz's zlib-compatible
API, two Zopfli copies and so on can live in one binary (see `build.rs`, `build/units.rs` and
`csrc/fz.h`).

## Verified bit-exactness

These checks were run on arm64 macOS against a corpus of text, binaries, random data,
repetitive data, a 1-byte file and a 5 MB file:

- `zlib`: matches Python's system zlib (1.2.12) on 1050 combinations of level 1–9, wbits, mem
  and strategy. It also matches zlib's own `examples/zpipe.c` at levels 0, 1, 6 and 9 when
  run with `inbuf=16384,outbuf=16384`.
- `zlib-ng`: the generic build matches an optimized NEON build made with zlib-ng's own
  `configure` at levels 0–9.
- `gzip` / `infozip`: match the real `gzip -N` and `zip -N` built from the same sources, at levels 1–9.
- `isa-l`: matches the `igzip` CLI built from the same base sources, at levels 0–3.
- `7zip`: matches a real `7zz` (Alone2) on `-mx`, `-mfb`, `-mpass`, `-mmc` and `-ma` variations.
- `advancecomp-7z`: matches AdvanceCOMP's `compress_rfc1950_7z`.
- `ect`: matches ECT's own `ZopfliGzip` path at levels 1–9 and with iteration overrides.
- `fflate`, `pako`, `uzip`, `zlibjs`: match node running the same vendored files. `pako`
  (a zlib 1.2.8 port) is byte-identical to `zlib` in every case tried.
- `cryptopp`: no reference comparison yet; it uses the unmodified upstream `Deflator`.

## Caveats worth knowing

- **Stored blocks depend on how the library is called.** zlib's `deflate_stored` sizes blocks
  after `avail_in`/`avail_out`. That is why the C zlib family has `inbuf`/`outbuf`
  parameters (0 = one-shot, like `compress2`). Python's `zlib.compress(level=0)`, for
  example, grows its output buffer and so produces different stored blocks. ISA-L's
  `chunk_kib` is a similar knob.
- **Some output depends on the CPU or the build.** Cloudflare's hash depends on the target,
  hence the two variants. ISA-L is built from its portable C, and its asm paths are not
  documented to produce the same streams. ECT's level 1 uses a Chromium-derived zlib with CRC
  hashing. The output of `go` follows the installed Go toolchain.
- stb, sdefl, lodepng (stored or fixed) and uzip level 0 emit nothing at all for empty input. This
  is surfaced as `Error::Encode`, and `apple` rejects empty input. zlibjs `type=none|fixed` emits
  a truncated stream for empty input (node does the same), so the round-trip check fails there.
- The JS backends are slow: pako level 9 and zlibjs with `lazy` take seconds per MB.
- Several backends turn out to be bit-identical on most inputs: `zlib-ng`/`zlib-rs`,
  `miniz`/`miniz-oxide`, and often `zlib`/`zlib-chromium-rk`. `flate-zoo survey` shows the
  equivalence classes for a given file.

## Not covered (yet)

Zig's `std.compress.flate`, .NET's and Java's managed ports (they are zlib or zlib-ng
underneath anyway), Nim zippy, OCaml decompress, PuTTY's own encoder (uzlib is derived from it),
older zlib releases, and closed-source tools (kzip, PNGOUT, PKZIP, WinRAR).

## Parting matter

The code proper in this repository is released into the public domain (or MIT-0 license).
**This does not mean that you are completely free to use this crate though!** For example,
the `gzip` backend compiles GNU gzip, which is GPL-3, so any binary built with the default
features is GPL-3. Build with `--no-default-features` and a feature list without `gzip`
if that matters.

Claude Opus 5.5 was used to one-shot this crate out of thin air. The purported author,
Kang Seonghoon, only gave a single-line prompt (and this paragraph). What a life.
