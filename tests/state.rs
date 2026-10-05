//! A call's output must not depend on anything but its input and parameters: not on what ran
//! earlier on the same thread, nor on leftover stack contents. Violations of the latter are
//! usually reads of uninitialized memory, which may also crash.

use flate_zoo::{Encoder, Sweep, backends, presets_of};
use std::hint::black_box;

fn input() -> Vec<u8> {
    include_bytes!("../src/lib.rs").repeat(2)
}

/// Runs `f` on a fresh thread, whose stack starts out zeroed.
fn fresh<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(8 << 20)
            .spawn_scoped(s, f)
            .unwrap()
            .join()
            .unwrap()
    })
}

/// Fills the next 1 MiB of stack below the caller with `byte`.
#[inline(never)]
fn dirty_stack(byte: u8) {
    let mut junk = [0u8; 1 << 20];
    black_box(&mut junk).fill(byte);
    black_box(&junk);
}

fn presets() -> Vec<Encoder> {
    backends()
        .iter()
        .flat_map(|&b| presets_of(b, Sweep::Default))
        .collect()
}

#[test]
fn independent_of_stack_garbage() {
    let input = input();
    for enc in presets() {
        let clean = fresh(|| enc.compress(&input));
        for byte in [0x5a, 0xff] {
            let dirty = fresh(|| {
                dirty_stack(byte);
                enc.compress(&input)
            });
            assert_eq!(
                clean, dirty,
                "{enc}: output depends on stack contents ({byte:#x})"
            );
        }
    }
}
