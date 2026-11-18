//! Registers every module in src/jobs/ as a job, so adding a job never touches a shared file.
use std::{env, fs, path::Path};

fn main() {
    let jobs_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/jobs");
    println!("cargo:rerun-if-changed={}", jobs_dir.display());
    let mut names: Vec<String> = fs::read_dir(&jobs_dir)
        .unwrap()
        .filter_map(|e| {
            let path = e.unwrap().path();
            (path.extension()? == "rs")
                .then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    names.sort();

    let mut out = String::new();
    for name in &names {
        out += &format!(
            "#[path = {:?}]\npub mod {name};\n",
            jobs_dir.join(format!("{name}.rs"))
        );
    }
    out += "\n/// All ported jobs, by name.\npub const REGISTRY: &[(&str, crate::Job)] = &[\n";
    for name in &names {
        out += &format!("    ({name:?}, {name}::run),\n");
    }
    out += "];\n";
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("jobs.rs"),
        out,
    )
    .unwrap();
}
