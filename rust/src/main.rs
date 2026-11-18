//! Usage: telemetry-job <events.jsonl> <output-dir> [job ...]   (no jobs = all ported jobs)
use std::{env, fs, path::Path, process};
use telemetry::{jobs::REGISTRY, read_events};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--list") {
        REGISTRY.iter().for_each(|(name, _)| println!("{name}"));
        return;
    }
    if args.len() < 2 {
        eprintln!("usage: telemetry-job <events.jsonl> <output-dir> [job ...] | --list");
        process::exit(2);
    }
    let events = read_events(Path::new(&args[0])).expect("read events");
    let out_dir = Path::new(&args[1]);
    fs::create_dir_all(out_dir).expect("create output dir");

    let requested = &args[2..];
    for name in requested {
        if !REGISTRY.iter().any(|(n, _)| n == name) {
            eprintln!("job not ported yet: {name}");
            process::exit(2);
        }
    }
    for (name, job) in REGISTRY {
        if !requested.is_empty() && !requested.iter().any(|r| r == name) {
            continue;
        }
        let mut lines: Vec<String> = job(&events).iter().map(|v| v.to_string()).collect();
        lines.sort();
        fs::write(
            out_dir.join(format!("{name}.jsonl")),
            lines.join("\n") + "\n",
        )
        .expect("write output");
        println!("{name:<28} {:>6} records", lines.len());
    }
}
