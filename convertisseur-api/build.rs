//! Re-runs the build when the migrations or the content files change, and
//! lists the content files for `include_str!`.

use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=migrations");
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let content_dir = manifest_dir.join("../content");
    println!("cargo::rerun-if-changed={}", content_dir.display());
    embed_content(&content_dir);
}

/// Writes `$OUT_DIR/content_files.rs`: every `.yaml` file under `content/`,
/// sorted by path, as `(path relative to content/, text)`.
fn embed_content(content_dir: &Path) {
    let content_dir = content_dir
        .canonicalize()
        .unwrap_or_else(|error| panic!("{}: {error}", content_dir.display()));
    let mut files = Vec::new();
    collect_yaml(&content_dir, &mut files);
    files.sort();
    let mut code = String::from("&[\n");
    for path in files {
        let relative = path
            .strip_prefix(&content_dir)
            .unwrap()
            .components()
            .map(|c| c.as_os_str().to_str().unwrap())
            .collect::<Vec<_>>()
            .join("/");
        writeln!(
            code,
            "    ({relative:?}, include_str!({:?})),",
            path.to_str().unwrap()
        )
        .unwrap();
    }
    code.push(']');
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out_dir.join("content_files.rs"), code).unwrap();
}

fn collect_yaml(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_yaml(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "yaml") {
            files.push(path);
        }
    }
}
