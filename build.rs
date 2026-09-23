use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn collect(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, files);
        } else if path.is_file() {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
}

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let assets = root.join("assets");
    let mut files = Vec::new();
    collect(&assets, &assets, &mut files);
    files.sort();

    let mut generated = String::from("pub const EMBEDDED_PATHS: &[&str] = &[\n");
    for relative in &files {
        let key = format!("assets/{}", relative.to_string_lossy().replace('\\', "/"));
        println!("cargo:rerun-if-changed={}", assets.join(relative).display());
        writeln!(generated, "    {:?},", key).unwrap();
    }
    generated
        .push_str("];\n\npub fn bytes(path: &str) -> Option<&'static [u8]> {\n    match path {\n");
    for relative in files {
        let key = format!("assets/{}", relative.to_string_lossy().replace('\\', "/"));
        let source = assets.join(&relative);
        writeln!(
            generated,
            "        {:?} => Some(include_bytes!({:?}).as_slice()),",
            key, source
        )
        .unwrap();
    }
    generated.push_str("        _ => None,\n    }\n}\n");

    let output = PathBuf::from(std::env::var_os("OUT_DIR").expect("out dir"));
    std::fs::write(output.join("embedded_assets.rs"), generated).expect("write embedded assets");
}
