use flate_zoo::{Encoder, Error, Sweep, backends, inflate, presets_of};

fn inputs() -> Vec<Vec<u8>> {
    let mut lcg = 0x1234_5678u32;
    let noise: Vec<u8> = (0..70_000)
        .map(|_| {
            lcg = lcg.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (lcg >> 24) as u8
        })
        .collect();
    let text = include_bytes!("../src/lib.rs").repeat(4);
    vec![
        vec![],
        b"a".to_vec(),
        b"abcabcabd".repeat(20_000),
        noise,
        text,
    ]
}

#[test]
fn every_backend_round_trips() {
    let inputs = inputs();
    for &b in backends() {
        for enc in presets_of(b, Sweep::Default) {
            for input in &inputs {
                match enc.compress(input) {
                    Ok(raw) => assert_eq!(
                        inflate(&raw).as_deref(),
                        Ok(&input[..]),
                        "{enc} on {} bytes",
                        input.len()
                    ),
                    // Several libraries cannot encode empty input; nothing else may fail.
                    Err(Error::Encode(_)) if input.is_empty() => {}
                    Err(e) => panic!("{enc} on {} bytes: {e}", input.len()),
                }
            }
        }
    }
}

#[test]
fn specs_round_trip() {
    for e in flate_zoo::presets(Sweep::OneAtATime) {
        assert_eq!(e.to_string().parse::<Encoder>().unwrap(), e);
        assert_eq!(e.short().parse::<Encoder>().unwrap(), e);
    }
    assert!("zlib:level=10".parse::<Encoder>().is_err());
    assert!("zlib:nope=1".parse::<Encoder>().is_err());
    assert!("nope".parse::<Encoder>().is_err());
    let e: Encoder = "zlib:strategy=rle".parse().unwrap();
    assert_eq!(e.get("strategy"), Some(3));
}
