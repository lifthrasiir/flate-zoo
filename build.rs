//! Builds every vendored C/C++ encoder as a separate static library.
//!
//! Many of them (zlib and its forks, miniz in zlib-compatible mode, ...) export the very same
//! global symbols, so each C unit is compiled twice: the first pass only collects the global
//! symbols it defines, and the second pass force-includes a generated header that renames
//! all of them to `fz_<unit>_<symbol>`. The shim (`csrc/*.c`) of each unit is compiled with the
//! same header and exposes the unit-specific entry points declared in `src/ffi.rs`.

use std::collections::BTreeSet;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "build/units.rs"]
mod units;

pub struct Unit {
    /// Unit name; also the symbol prefix after replacing `-` with `_`.
    pub name: &'static str,
    /// Cargo feature that enables this unit; defaults to `name`.
    pub feature: &'static str,
    /// Library sources. Their global symbols are prefixed when `prefix` is set.
    pub srcs: Vec<PathBuf>,
    /// Entry point sources. Not scanned for symbols, but compiled with the prefix header.
    pub shims: Vec<PathBuf>,
    pub includes: Vec<PathBuf>,
    pub defines: Vec<(&'static str, Option<String>)>,
    pub flags: Vec<&'static str>,
    pub cpp: bool,
    /// Compile `.cpp` files as C++ and the rest as C within one unit (symbols prefixed jointly).
    pub mixed: bool,
    pub prefix: bool,
    /// Extra libraries to link (`kind=name` or `name`).
    pub link: Vec<&'static str>,
}

impl Unit {
    pub fn new(name: &'static str) -> Self {
        Unit {
            name,
            feature: name,
            srcs: vec![],
            shims: vec![],
            includes: vec![],
            defines: vec![],
            flags: vec![],
            cpp: false,
            mixed: false,
            prefix: true,
            link: vec![],
        }
    }
    pub fn ident(&self) -> String {
        self.name.replace('-', "_")
    }
    pub fn feature(mut self, feature: &'static str) -> Self {
        self.feature = feature;
        self
    }
    pub fn src(mut self, dir: &str, files: &[&str]) -> Self {
        self.srcs
            .extend(files.iter().map(|f| Path::new(dir).join(f)));
        self
    }
    pub fn file(mut self, path: PathBuf) -> Self {
        self.srcs.push(path);
        self
    }
    pub fn shim(mut self, file: &str) -> Self {
        self.shims.push(file.into());
        self
    }
    pub fn inc(mut self, dir: impl Into<PathBuf>) -> Self {
        self.includes.push(dir.into());
        self
    }
    pub fn def(mut self, k: &'static str, v: Option<&str>) -> Self {
        self.defines.push((k, v.map(str::to_owned)));
        self
    }
    pub fn flag(mut self, f: &'static str) -> Self {
        self.flags.push(f);
        self
    }
    pub fn cpp(mut self) -> Self {
        self.cpp = true;
        self
    }
    pub fn mixed(mut self) -> Self {
        self.mixed = true;
        self
    }
    pub fn no_prefix(mut self) -> Self {
        self.prefix = false;
        self
    }
    pub fn link(mut self, l: &'static str) -> Self {
        self.link.push(l);
        self
    }
}

pub fn out_dir() -> PathBuf {
    PathBuf::from(env::var_os("OUT_DIR").unwrap())
}

pub fn feature(name: &str) -> bool {
    let var = format!("CARGO_FEATURE_{}", name.to_uppercase().replace('-', "_"));
    env::var_os(var).is_some()
}

fn target_is_apple() -> bool {
    env::var("CARGO_CFG_TARGET_VENDOR").as_deref() == Ok("apple")
}

fn base_build(u: &Unit, out: &Path, cpp: bool) -> cc::Build {
    let mut b = cc::Build::new();
    b.cpp(cpp)
        .opt_level(2)
        .debug(false)
        .warnings(false)
        .cargo_warnings(false)
        .out_dir(out)
        .include("csrc")
        .define("FZ_UNIT", Some(u.ident().as_str()));
    for i in &u.includes {
        b.include(i);
    }
    for (k, v) in &u.defines {
        b.define(k, v.as_deref());
    }
    for f in &u.flags {
        b.flag(f);
    }
    b
}

/// Global symbols defined by the given objects, without the platform's C symbol prefix.
fn defined_symbols(objs: &[PathBuf]) -> BTreeSet<String> {
    let nm = env::var("NM").unwrap_or_else(|_| "nm".into());
    let output = Command::new(&nm)
        .arg("-g")
        .args(objs)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {nm}: {e}"));
    assert!(
        output.status.success(),
        "{nm} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let strip = if target_is_apple() { "_" } else { "" };
    let mut syms = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [_, kind, name] = fields[..] else {
            continue;
        };
        if matches!(kind, "U" | "w" | "v") {
            continue;
        }
        let Some(name) = name.strip_prefix(strip) else {
            continue;
        };
        if name.starts_with("_Z") || !name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()) {
            continue;
        }
        syms.insert(name.to_owned());
    }
    syms
}

fn build_unit(u: &Unit) {
    let out = out_dir().join("units").join(u.name);
    fs::create_dir_all(&out).unwrap();
    for p in u.srcs.iter().chain(&u.shims) {
        assert!(
            p.exists(),
            "{}: missing source {} (run ./fetch.sh?)",
            u.name,
            p.display()
        );
    }

    // Plain units: all sources compiled with `u.cpp`. Mixed units: `.cpp`/`.cc`/`.cxx` files are
    // compiled as C++ and everything else as C, in separate builds that share the prefix header.
    let is_cxx = |p: &&PathBuf| {
        p.extension()
            .is_some_and(|e| e == "cpp" || e == "cc" || e == "cxx")
    };
    let mut builds: Vec<(bool, Vec<&PathBuf>, Vec<&PathBuf>, &str)> = vec![];
    if u.mixed {
        let (x_srcs, c_srcs): (Vec<_>, Vec<_>) = u.srcs.iter().partition(is_cxx);
        let (x_shims, c_shims): (Vec<_>, Vec<_>) = u.shims.iter().partition(is_cxx);
        builds.push((true, x_srcs, x_shims, ""));
        if !c_srcs.is_empty() || !c_shims.is_empty() {
            builds.push((false, c_srcs, c_shims, "_c"));
        }
    } else {
        builds.push((u.cpp, u.srcs.iter().collect(), u.shims.iter().collect(), ""));
    }
    let header = if u.prefix {
        let pass1 = out.join("pass1");
        let mut objs = vec![];
        for (cpp, srcs, _, suffix) in &builds {
            let mut b1 = base_build(u, &pass1.join(suffix), *cpp);
            b1.files(srcs.iter().copied());
            objs.extend(b1.compile_intermediates());
        }
        let mut h = format!(
            "/* generated by build.rs for unit {} */\n#pragma once\n",
            u.name
        );
        for s in defined_symbols(&objs) {
            writeln!(h, "#define {s} fz_{}_{s}", u.ident()).unwrap();
        }
        let header = out.join("fz_prefix.h");
        fs::write(&header, h).unwrap();
        Some(header)
    } else {
        None
    };
    for (cpp, srcs, shims, suffix) in &builds {
        let mut b = base_build(u, &out.join("final").join(suffix), *cpp);
        if let Some(header) = &header {
            b.flag("-include").flag(header.to_str().unwrap());
        }
        b.files(srcs.iter().copied()).files(shims.iter().copied());
        b.compile(&format!("fz_{}{suffix}", u.ident()));
    }
    for l in &u.link {
        println!("cargo:rustc-link-lib={l}");
    }
}

fn build_go() {
    let out = out_dir().join("go");
    fs::create_dir_all(&out).unwrap();
    let go = env::var("GO").unwrap_or_else(|_| "go".into());
    let status = Command::new(&go)
        .current_dir("go")
        .env("CGO_ENABLED", "1")
        .env("GOFLAGS", "-mod=vendor")
        .args(["build", "-trimpath", "-buildmode=c-archive", "-o"])
        .arg(out.join("libfz_go.a"))
        .arg(".")
        .status()
        .unwrap_or_else(|e| panic!("cannot run {go}: {e}"));
    assert!(status.success(), "go build failed");
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=fz_go");
    if target_is_apple() {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=resolv");
    }
    println!("cargo:rerun-if-changed=go");
}

fn main() {
    println!("cargo:rerun-if-changed=build");
    println!("cargo:rerun-if-changed=csrc");
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-env-changed=NM");
    println!("cargo:rerun-if-env-changed=GO");

    let units: Vec<Unit> = units::all()
        .into_iter()
        .filter(|u| feature(u.feature))
        .collect();
    std::thread::scope(|s| {
        for u in &units {
            s.spawn(|| build_unit(u));
        }
        if feature("go") {
            s.spawn(build_go);
        }
    });
}
