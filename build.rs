use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let directory = root.join("src/algorithms");
    // Watching the directory also catches added/deleted files, not just edits.
    println!("cargo:rerun-if-changed=src/algorithms");
    let mut files: Vec<_> = fs::read_dir(&directory)
        .expect("cannot read src/algorithms")
        .map(|entry| entry.expect("cannot read algorithm entry").path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "rs"))
        .filter(|p| {
            let stem = p.file_stem().unwrap().to_string_lossy();
            stem != "mod" && !stem.starts_with('_')
        })
        .collect();
    files.sort();
    assert!(
        !files.is_empty(),
        "add an algorithm with pub fn create() -> Box<dyn Algorithm> in src/algorithms"
    );
    let mut code = String::new();
    for (index, file) in files.iter().enumerate() {
        let path = file.to_str().expect("algorithm path must be UTF-8");
        code.push_str(&format!("#[path = {path:?}] mod algorithm_{index};\n"));
    }
    code.push_str("pub const ALGORITHMS: &[fn() -> Box<dyn crate::api::Algorithm>] = &[\n");
    for index in 0..files.len() {
        code.push_str(&format!("algorithm_{index}::create,\n"));
    }
    code.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("algorithms.rs"),
        code,
    )
    .expect("cannot write algorithm registry");
}
