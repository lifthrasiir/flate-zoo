//! Backends implemented by foreign code following the `csrc/fz.h` calling convention.

use crate::{Backend, Error, Param, Result};
use std::ffi::{c_int, c_longlong, c_uchar};

pub type EntryFn = unsafe extern "C" fn(
    input: *const c_uchar,
    in_len: usize,
    params: *const c_longlong,
    nparams: usize,
    out: *mut *mut c_uchar,
    out_len: *mut usize,
) -> c_int;

unsafe extern "C" {
    fn free(p: *mut std::ffi::c_void);
}

pub struct Native {
    pub name: &'static str,
    pub about: &'static str,
    pub params: &'static [Param],
    pub entry: EntryFn,
    /// Serializes calls into libraries with global state.
    pub lock: Option<&'static std::sync::Mutex<()>>,
}

impl Backend for Native {
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
        let _guard = self
            .lock
            .map(|m| m.lock().unwrap_or_else(|e| e.into_inner()));
        let mut out = std::ptr::null_mut();
        let mut out_len = 0;
        // SAFETY: the entry point reads exactly `input.len()` bytes and `values.len()` params,
        // and on success hands over a malloc'ed buffer of `out_len` bytes.
        let rc = unsafe {
            (self.entry)(
                input.as_ptr(),
                input.len(),
                values.as_ptr(),
                values.len(),
                &mut out,
                &mut out_len,
            )
        };
        if rc != 0 {
            let what = match rc {
                -1 => "rejected parameters",
                -2 => "out of memory",
                _ => "library error",
            };
            return Err(if rc == -1 {
                Error::Param(format!("{}: {what}", self.name))
            } else {
                Error::Encode(format!("{}: {what} ({rc})", self.name))
            });
        }
        if out_len == 0 {
            unsafe { free(out.cast()) };
            // stb, sdefl and lodepng do this for empty inputs; it is not a valid DEFLATE stream.
            return Err(Error::Encode(format!(
                "{}: library produced no output",
                self.name
            )));
        }
        // SAFETY: see above.
        let v = unsafe { std::slice::from_raw_parts(out, out_len).to_vec() };
        unsafe { free(out.cast()) };
        Ok(v)
    }
}

/// Declares a static [`Native`] backend for the entry point `fz_<sym>_run`.
macro_rules! native {
    ($vis:vis $static:ident = $sym:ident($name:literal, $about:literal, $params:expr $(, lock = $lock:expr)?)) => {
        $vis static $static: $crate::native::Native = {
            unsafe extern "C" {
                fn $sym(
                    input: *const std::ffi::c_uchar,
                    in_len: usize,
                    params: *const std::ffi::c_longlong,
                    nparams: usize,
                    out: *mut *mut std::ffi::c_uchar,
                    out_len: *mut usize,
                ) -> std::ffi::c_int;
            }
            $crate::native::Native {
                name: $name,
                about: $about,
                params: $params,
                entry: $sym,
                lock: { #[allow(unused_mut, unused_assignments)] let mut l = None; $(l = Some($lock);)? l },
            }
        };
    };
}
pub(crate) use native;
