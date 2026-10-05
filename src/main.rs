use flate_zoo::{Encoder, Sweep, backends, inflate, presets_of};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::ExitCode;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const USAGE: &str = "\
usage:
  flate-zoo list
      List backends and their parameters.
  flate-zoo compress SPEC [INPUT [OUTPUT]]
      Raw DEFLATE-compress INPUT (default stdin) to OUTPUT (default stdout).
      SPEC is `backend[:param=value,...]`, e.g. `zlib:level=9,strategy=filtered`.
  flate-zoo survey [-s SWEEP] [-b BACKEND]... FILE
      Run every preset on FILE, verify round trips and group identical outputs.
  flate-zoo identify [-s SWEEP] [-b BACKEND]... RAW_DEFLATE_FILE
      Find presets that reproduce the given raw DEFLATE stream bit-exactly.

SWEEP is `default`, `one` (one parameter at a time; the default) or `full` (cartesian product).
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let res = match args.first().map(String::as_str) {
        Some("list") => list(),
        Some("compress") => compress(&args[1..]),
        Some("survey") => survey(&args[1..], false),
        Some("identify") => survey(&args[1..], true),
        _ => {
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match res {
        Ok(code) => code,
        Err(e) => {
            eprintln!("flate-zoo: {e}");
            ExitCode::FAILURE
        }
    }
}

type Res = Result<ExitCode, Box<dyn std::error::Error>>;

fn list() -> Res {
    for b in backends() {
        println!("{:<24} {}", b.name(), b.about());
        for p in b.params() {
            let names: Vec<String> = p.names.iter().map(|(n, v)| format!("{n}={v}")).collect();
            let names = if names.is_empty() {
                String::new()
            } else {
                format!(" [{}]", names.join(" "))
            };
            println!(
                "    {:<12} default {:<8} {}..={}{}  {}",
                p.name, p.default, p.min, p.max, names, p.help
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn compress(args: &[String]) -> Res {
    let spec = args.first().ok_or("missing SPEC")?;
    let enc: Encoder = spec.parse()?;
    let input = match args.get(1).map(String::as_str) {
        None | Some("-") => {
            let mut v = Vec::new();
            std::io::stdin().read_to_end(&mut v)?;
            v
        }
        Some(path) => std::fs::read(path)?,
    };
    let out = enc.compress(&input)?;
    match args.get(2).map(String::as_str) {
        None | Some("-") => std::io::stdout().write_all(&out)?,
        Some(path) => std::fs::write(path, &out)?,
    }
    Ok(ExitCode::SUCCESS)
}

struct Outcome {
    enc: Encoder,
    result: Result<Vec<u8>, String>,
    time: Duration,
}

fn survey(args: &[String], identify: bool) -> Res {
    let mut sweep = Sweep::OneAtATime;
    let mut only: Vec<String> = vec![];
    let mut file = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-s" => sweep = it.next().ok_or("-s needs a value")?.parse()?,
            "-b" => only.push(it.next().ok_or("-b needs a value")?.clone()),
            _ if file.is_none() => file = Some(a),
            _ => return Err(format!("unexpected argument {a:?}").into()),
        }
    }
    let file = std::fs::read(file.ok_or("missing FILE")?)?;
    let (input, target) = if identify {
        (inflate(&file)?, Some(file))
    } else {
        (file, None)
    };
    for b in &only {
        flate_zoo::backend(b).ok_or_else(|| format!("unknown backend {b:?}"))?;
    }
    let encoders: Vec<Encoder> = backends()
        .iter()
        .filter(|b| only.is_empty() || only.iter().any(|o| o == b.name()))
        .flat_map(|&b| presets_of(b, sweep))
        .collect();

    let next = AtomicUsize::new(0);
    let outcomes = Mutex::new(Vec::with_capacity(encoders.len()));
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(enc) = encoders.get(i) else { break };
                    let t = Instant::now();
                    let result = enc
                        .compress(&input)
                        .map_err(|e| e.to_string())
                        .and_then(|out| match inflate(&out) {
                            Ok(back) if back == input => Ok(out),
                            Ok(_) => Err("round trip mismatch".to_owned()),
                            Err(e) => Err(format!("output does not decode: {e}")),
                        });
                    let time = t.elapsed();
                    outcomes.lock().unwrap().push(Outcome {
                        enc: enc.clone(),
                        result,
                        time,
                    });
                }
            });
        }
    });
    let mut outcomes = outcomes.into_inner().unwrap();
    outcomes.sort_by_key(|o| encoders.iter().position(|e| *e == o.enc));

    let mut failed = 0;
    for o in &outcomes {
        if let Err(e) = &o.result {
            failed += 1;
            eprintln!("FAIL {}: {e}", o.enc.short());
        }
    }

    if let Some(target) = target {
        let mut hits = 0;
        let mut best: Option<(usize, &Outcome)> = None;
        for o in &outcomes {
            let Ok(out) = &o.result else { continue };
            if *out == target {
                hits += 1;
                println!("MATCH {}", o.enc.short());
            } else {
                let common = out.iter().zip(&target).take_while(|(a, b)| a == b).count();
                if best.is_none_or(|(c, _)| common > c) {
                    best = Some((common, o));
                }
            }
        }
        if hits == 0 {
            if let Some((common, o)) = best {
                println!(
                    "no exact match; longest common prefix {common}/{} bytes with {}",
                    target.len(),
                    o.enc.short()
                );
            }
            return Ok(ExitCode::FAILURE);
        }
        return Ok(ExitCode::SUCCESS);
    }

    // Group identical outputs; each class is listed once, ordered by size.
    let mut classes: BTreeMap<&[u8], Vec<&Outcome>> = BTreeMap::new();
    for o in &outcomes {
        if let Ok(out) = &o.result {
            classes.entry(out).or_default().push(o);
        }
    }
    let mut classes: Vec<_> = classes.into_iter().collect();
    classes.sort_by_key(|(out, members)| (out.len(), members[0].enc.to_string()));
    println!("{:>12} {:>8} {:>10}  members", "size", "ratio", "best ms");
    for (out, members) in &classes {
        let ms = members.iter().map(|o| o.time).min().unwrap().as_secs_f64() * 1e3;
        let names: Vec<String> = members.iter().map(|o| o.enc.short()).collect();
        println!(
            "{:>12} {:>7.3}% {:>10.2}  {}",
            out.len(),
            out.len() as f64 * 100.0 / input.len().max(1) as f64,
            ms,
            names.join(" ")
        );
    }
    let ok = outcomes.len() - failed;
    let backends_hit = outcomes
        .iter()
        .filter(|o| o.result.is_ok())
        .map(|o| o.enc.backend().name())
        .collect::<std::collections::BTreeSet<_>>();
    eprintln!(
        "{} presets over {} backends, {ok} ok, {failed} failed, {} distinct outputs",
        outcomes.len(),
        backends_hit.len(),
        classes.len()
    );
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
