//! The wasm2c translation (`go`, `go-klauspost`) must be bit-identical to the c-archive it
//! stands in for (`go-native`, `go-klauspost-native`).
#![cfg(all(feature = "go", feature = "go-native"))]

use flate_zoo::{Encoder, Sweep, backend, presets_of};

fn inputs() -> Vec<Vec<u8>> {
    let mut lcg = 0x1234_5678u32;
    let mut noise = |n: usize| -> Vec<u8> {
        (0..n)
            .map(|_| {
                lcg = lcg.wrapping_mul(1_103_515_245).wrapping_add(12345);
                (lcg >> 24) as u8
            })
            .collect()
    };
    let text = include_bytes!("../src/lib.rs");
    // Text with sparse noise: long matches, broken often enough to need many blocks.
    let mut mixed = text.repeat(40);
    let n = mixed.len();
    for (i, b) in noise(n / 97).into_iter().enumerate() {
        mixed[i * 97] = b;
    }
    vec![
        vec![],
        b"a".to_vec(),
        b"abcabcabd".repeat(20_000),
        noise(70_000),
        text.repeat(4),
        mixed,
        // larger than klauspost's and Go's internal buffers, half of it incompressible
        [noise(1 << 20), text.repeat(1 << 20 >> 13)].concat(),
    ]
}

#[test]
fn same_as_native() {
    let inputs = inputs();
    for (port, native) in [("go", "go-native"), ("go-klauspost", "go-klauspost-native")] {
        let native = backend(native).unwrap();
        for enc in presets_of(backend(port).unwrap(), Sweep::OneAtATime) {
            let mut reference = Encoder::new(native);
            for (p, &v) in enc.backend().params().iter().zip(enc.values()) {
                reference = reference.set(p.name, v).unwrap();
            }
            for input in &inputs {
                assert!(
                    enc.compress(input) == reference.compress(input),
                    "{enc} differs from {reference} on {} bytes",
                    input.len()
                );
            }
        }
    }
}
