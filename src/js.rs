//! JavaScript DEFLATE implementations, run in-process on an embedded QuickJS engine.
//!
//! Each library gets its own lazily created QuickJS runtime and context per thread (contexts
//! are not `Sync`). Inputs and outputs travel as typed arrays.

use crate::{Backend, Error, Param, Result};
use rquickjs::{Context, Ctx, Function, Runtime, TypedArray};
use std::cell::RefCell;
use std::thread::LocalKey;

use crate::backends::ZLIB_PARAMS;

struct Engine {
    ctx: Context,
    // Keeps the runtime alive as long as the context.
    _rt: Runtime,
}

type Slot = RefCell<Option<Engine>>;

pub struct JsBackend {
    name: &'static str,
    about: &'static str,
    params: &'static [Param],
    /// The library source followed by the glue defining `__run(input, ...params)`.
    sources: &'static [&'static str],
    slot: &'static LocalKey<Slot>,
}

fn js_err(name: &str, e: rquickjs::Error, ctx: Option<&Ctx<'_>>) -> Error {
    let detail = match (e, ctx) {
        (rquickjs::Error::Exception, Some(ctx)) => format!("{:?}", ctx.catch()),
        (e, _) => e.to_string(),
    };
    Error::Encode(format!("{name}: {detail}"))
}

impl JsBackend {
    fn engine(&self) -> Result<Engine> {
        let rt = Runtime::new().map_err(|e| js_err(self.name, e, None))?;
        // Inputs of several MiB are fine, but be explicit that there is no memory limit.
        rt.set_memory_limit(0);
        let ctx = Context::full(&rt).map_err(|e| js_err(self.name, e, None))?;
        ctx.with(|ctx| {
            for src in self.sources {
                ctx.eval::<(), _>(*src)
                    .map_err(|e| js_err(self.name, e, Some(&ctx)))?;
            }
            Ok(())
        })?;
        Ok(Engine { ctx, _rt: rt })
    }
}

impl Backend for JsBackend {
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
        self.slot.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot.is_none() {
                *slot = Some(self.engine()?);
            }
            let engine = slot.as_ref().unwrap();
            engine.ctx.with(|ctx| {
                let run = || -> rquickjs::Result<Vec<u8>> {
                    let f: Function = ctx.globals().get("__run")?;
                    let data = TypedArray::<u8>::new_copy(ctx.clone(), input)?;
                    let mut args = rquickjs::function::Args::new(ctx.clone(), 1 + values.len());
                    args.push_arg(data)?;
                    for &v in values {
                        args.push_arg(v as i32)?;
                    }
                    let out: TypedArray<u8> = args.apply(&f)?;
                    // SAFETY: no JavaScript runs while the borrowed slice is alive.
                    let bytes = unsafe { out.as_bytes() }
                        .ok_or(rquickjs::Error::new_from_js("typed array", "bytes"))?;
                    Ok(bytes.to_vec())
                };
                let r = run().map_err(|e| js_err(self.name, e, Some(&ctx)));
                // Drop garbage (input copies, big intermediate buffers) eagerly.
                ctx.run_gc();
                r
            })
        })
    }
}

macro_rules! js_backend {
    ($ident:ident, $tls:ident, $name:literal, $about:literal, $params:expr, [$($src:expr),+ $(,)?]) => {
        thread_local! {
            static $tls: Slot = const { RefCell::new(None) };
        }
        pub static $ident: JsBackend = JsBackend {
            name: $name,
            about: $about,
            params: $params,
            sources: &[$($src),+],
            slot: &$tls,
        };
    };
}

js_backend!(
    FFLATE,
    FFLATE_TLS,
    "fflate",
    "fflate 0.8.2 (101arrowz) deflateSync on QuickJS; its own implementation",
    &[
        Param::new("level", 6, 0, 9, "compression level"),
        Param::new(
            "mem",
            -1,
            -1,
            12,
            "memory level (hash table size is 2^(mem+12)); -1 = let fflate choose from the input size"
        )
        .names(&[("auto", -1)])
        .sweep(&[-1, 0, 4, 8, 12]),
    ],
    [
        "var self = globalThis;",
        include_str!("../vendor/js-fflate/umd/index.js"),
        include_str!("../js/fflate.js"),
    ]
);

js_backend!(
    PAKO,
    PAKO_TLS,
    "pako",
    "pako 2.1.0 (nodeca) deflateRaw on QuickJS; a port of zlib 1.2.8",
    ZLIB_PARAMS,
    [
        include_str!("../vendor/js-pako/dist/pako_deflate.js"),
        include_str!("../js/pako.js"),
    ]
);

js_backend!(
    UZIP,
    UZIP_TLS,
    "uzip",
    "UZIP.js (photopea) deflateRaw on QuickJS; its own implementation",
    &[Param::new("level", 6, 0, 9, "compression level")],
    [
        include_str!("../vendor/js-uzip/UZIP.js"),
        include_str!("../js/uzip.js"),
    ]
);

js_backend!(
    ZLIBJS,
    ZLIBJS_TLS,
    "zlibjs",
    "zlib.js 0.3.1 (imaya) RawDeflate on QuickJS; its own implementation",
    &[
        Param::new(
            "type",
            2,
            0,
            2,
            "block type: stored, fixed Huffman or dynamic Huffman"
        )
        .names(&[("none", 0), ("fixed", 1), ("dynamic", 2)]),
        Param::new(
            "lazy",
            0,
            0,
            258,
            "lazy matching: a match shorter than this is deferred in favour of the next position's"
        )
        .sweep(&[0, 4, 8, 16, 32, 64, 128, 258]),
    ],
    [
        include_str!("../vendor/js-zlibjs/bin/rawdeflate.min.js"),
        include_str!("../js/zlibjs.js"),
    ]
);
