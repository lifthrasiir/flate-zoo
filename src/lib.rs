//! A zoo of DEFLATE encoders behind one interface.
//!
//! Every [`Backend`] turns bytes into a *raw* DEFLATE stream (RFC 1951, without zlib or gzip
//! framing); wrappers produced by the underlying library are stripped. A backend together
//! with concrete parameter values is an [`Encoder`], which has a textual spec form such as
//! `zlib:level=9,strategy=filtered` (omitted parameters take their defaults).
//!
//! ```no_run
//! let enc: flate_zoo::Encoder = "libdeflate:level=12".parse().unwrap();
//! let raw = enc.compress(b"hello hello hello").unwrap();
//! assert_eq!(flate_zoo::inflate(&raw).unwrap(), b"hello hello hello");
//! ```

use std::fmt;
use std::str::FromStr;

mod backends;
#[cfg(feature = "go")]
mod go;
#[cfg(feature = "js")]
mod js;
mod native;
#[cfg(feature = "rust")]
mod rust;

pub use backends::backends;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The spec string or parameter values were rejected.
    Param(String),
    /// The encoder failed (allocation failure, internal error, input too large...).
    Encode(String),
    /// The reference decoder rejected a stream.
    Decode(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Param(m) => write!(f, "invalid parameters: {m}"),
            Error::Encode(m) => write!(f, "encoding failed: {m}"),
            Error::Decode(m) => write!(f, "decoding failed: {m}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// An integer parameter of a backend. Enumerated parameters give names to their values.
#[derive(Debug, Clone, Copy)]
pub struct Param {
    pub name: &'static str,
    pub default: i64,
    pub min: i64,
    pub max: i64,
    /// Symbolic names for values, accepted when parsing and used when printing.
    pub names: &'static [(&'static str, i64)],
    /// Values tried by [`Sweep`]s; empty means every value in `min..=max`.
    pub sweep: &'static [i64],
    pub help: &'static str,
}

impl Param {
    pub const fn new(
        name: &'static str,
        default: i64,
        min: i64,
        max: i64,
        help: &'static str,
    ) -> Self {
        Param {
            name,
            default,
            min,
            max,
            names: &[],
            sweep: &[],
            help,
        }
    }
    pub const fn names(mut self, names: &'static [(&'static str, i64)]) -> Self {
        self.names = names;
        self
    }
    pub const fn sweep(mut self, sweep: &'static [i64]) -> Self {
        self.sweep = sweep;
        self
    }

    pub fn sweep_values(&self) -> Vec<i64> {
        if self.sweep.is_empty() {
            (self.min..=self.max).collect()
        } else {
            self.sweep.to_vec()
        }
    }

    fn parse(&self, s: &str) -> Result<i64> {
        let v = match self.names.iter().find(|(n, _)| *n == s) {
            Some(&(_, v)) => v,
            None => s
                .parse()
                .map_err(|_| Error::Param(format!("{}: cannot parse {s:?}", self.name)))?,
        };
        self.check(v)?;
        Ok(v)
    }

    fn check(&self, v: i64) -> Result<()> {
        if (self.min..=self.max).contains(&v) || self.names.iter().any(|&(_, n)| n == v) {
            Ok(())
        } else {
            Err(Error::Param(format!(
                "{}={v} is out of range {}..={}",
                self.name, self.min, self.max
            )))
        }
    }

    fn fmt_value(&self, v: i64) -> String {
        match self.names.iter().find(|&&(_, n)| n == v) {
            Some((n, _)) => (*n).to_owned(),
            None => v.to_string(),
        }
    }
}

/// A DEFLATE implementation.
pub trait Backend: Sync {
    /// Short identifier used in specs.
    fn name(&self) -> &'static str;
    /// One-line description including the upstream version.
    fn about(&self) -> &'static str;
    fn params(&self) -> &'static [Param];
    /// Compresses `input` into a raw DEFLATE stream. `values` match `params()` in order and
    /// have already been range-checked.
    fn compress(&self, input: &[u8], values: &[i64]) -> Result<Vec<u8>>;
}

impl fmt::Debug for dyn Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

pub fn backend(name: &str) -> Option<&'static dyn Backend> {
    backends().iter().copied().find(|b| b.name() == name)
}

/// A backend with fixed parameters.
#[derive(Clone)]
pub struct Encoder {
    backend: &'static dyn Backend,
    values: Vec<i64>,
}

impl Encoder {
    /// The encoder with default parameters.
    pub fn new(backend: &'static dyn Backend) -> Self {
        Encoder {
            backend,
            values: backend.params().iter().map(|p| p.default).collect(),
        }
    }

    pub fn backend(&self) -> &'static dyn Backend {
        self.backend
    }

    pub fn values(&self) -> &[i64] {
        &self.values
    }

    pub fn get(&self, name: &str) -> Option<i64> {
        let i = self.backend.params().iter().position(|p| p.name == name)?;
        Some(self.values[i])
    }

    pub fn set(mut self, name: &str, value: i64) -> Result<Self> {
        let params = self.backend.params();
        let i = params.iter().position(|p| p.name == name).ok_or_else(|| {
            Error::Param(format!("{} has no parameter {name:?}", self.backend.name()))
        })?;
        params[i].check(value)?;
        self.values[i] = value;
        Ok(self)
    }

    pub fn compress(&self, input: &[u8]) -> Result<Vec<u8>> {
        self.backend.compress(input, &self.values)
    }

    /// Spec string listing only the non-default parameters.
    pub fn short(&self) -> String {
        let mut s = self.backend.name().to_owned();
        let mut sep = ':';
        for (p, &v) in self.backend.params().iter().zip(&self.values) {
            if v != p.default {
                s.push(sep);
                s.push_str(p.name);
                s.push('=');
                s.push_str(&p.fmt_value(v));
                sep = ',';
            }
        }
        s
    }
}

/// The full spec string, listing every parameter.
impl fmt::Display for Encoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.backend.name())?;
        for (i, (p, &v)) in self.backend.params().iter().zip(&self.values).enumerate() {
            write!(
                f,
                "{}{}={}",
                if i == 0 { ':' } else { ',' },
                p.name,
                p.fmt_value(v)
            )?;
        }
        Ok(())
    }
}

impl fmt::Debug for Encoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl PartialEq for Encoder {
    fn eq(&self, other: &Self) -> bool {
        self.backend.name() == other.backend.name() && self.values == other.values
    }
}

impl Eq for Encoder {}

impl FromStr for Encoder {
    type Err = Error;

    /// Parses `name[:param=value,...]`.
    fn from_str(spec: &str) -> Result<Self> {
        let (name, rest) = spec.split_once(':').unwrap_or((spec, ""));
        let b = backend(name).ok_or_else(|| Error::Param(format!("unknown backend {name:?}")))?;
        let mut enc = Encoder::new(b);
        for kv in rest.split(',').filter(|s| !s.is_empty()) {
            let (k, v) = kv
                .split_once('=')
                .ok_or_else(|| Error::Param(format!("expected key=value, got {kv:?}")))?;
            let i = b
                .params()
                .iter()
                .position(|p| p.name == k)
                .ok_or_else(|| Error::Param(format!("{name} has no parameter {k:?}")))?;
            enc.values[i] = b.params()[i].parse(v)?;
        }
        Ok(enc)
    }
}

/// How thoroughly to enumerate parameter combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sweep {
    /// Defaults only.
    Default,
    /// Each parameter varied over its sweep values while the others stay at their defaults.
    OneAtATime,
    /// The cartesian product of all sweep values. Can be large (thousands for zlib forks).
    Full,
}

impl FromStr for Sweep {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "default" => Ok(Sweep::Default),
            "one" | "one-at-a-time" => Ok(Sweep::OneAtATime),
            "full" => Ok(Sweep::Full),
            _ => Err(Error::Param(format!(
                "unknown sweep {s:?} (default, one, full)"
            ))),
        }
    }
}

/// Parameter combinations of one backend, deduplicated, defaults first.
pub fn presets_of(backend: &'static dyn Backend, sweep: Sweep) -> Vec<Encoder> {
    let base = Encoder::new(backend);
    let mut out = vec![base.clone()];
    match sweep {
        Sweep::Default => {}
        Sweep::OneAtATime => {
            for (i, p) in backend.params().iter().enumerate() {
                for v in p.sweep_values() {
                    let mut e = base.clone();
                    e.values[i] = v;
                    out.push(e);
                }
            }
        }
        Sweep::Full => {
            let mut combos = vec![vec![]];
            for p in backend.params() {
                combos = combos
                    .into_iter()
                    .flat_map(|c: Vec<i64>| {
                        p.sweep_values().into_iter().map(move |v| {
                            let mut c = c.clone();
                            c.push(v);
                            c
                        })
                    })
                    .collect();
            }
            out.extend(combos.into_iter().map(|values| Encoder { backend, values }));
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|e| seen.insert(e.values.clone()));
    out
}

/// [`presets_of`] over every backend.
pub fn presets(sweep: Sweep) -> Vec<Encoder> {
    backends()
        .iter()
        .flat_map(|&b| presets_of(b, sweep))
        .collect()
}

/// Reference decoder for raw DEFLATE streams (miniz_oxide).
pub fn inflate(raw: &[u8]) -> Result<Vec<u8>> {
    miniz_oxide::inflate::decompress_to_vec(raw)
        .map_err(|e| Error::Decode(format!("{:?}", e.status)))
}
