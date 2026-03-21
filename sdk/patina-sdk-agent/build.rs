use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let root = manifest_dir.join("../..");
    let source = root.join("wit/toys");
    let out =
        PathBuf::from(std::env::var("OUT_DIR").expect("out dir")).join("patina-sdk-agent-wit/toys");

    let files = ["query.wit", "emit.wit", "layer.wit"];

    for file in files {
        let src = source.join(file);
        let dst = out.join(file);
        println!("cargo:rerun-if-changed={}", src.display());
        copy_if_changed(&src, &dst);
    }

    println!("cargo:rustc-env=PATINA_SDK_AGENT_WIT_DIR={}", out.display());
}

fn copy_if_changed(src: &Path, dst: &Path) {
    let bytes = fs::read(src).unwrap_or_else(|e| panic!("read {}: {}", src.display(), e));
    if let Ok(existing) = fs::read(dst) {
        if existing == bytes {
            return;
        }
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| panic!("mkdir {}: {}", parent.display(), e));
    }
    fs::write(dst, bytes).unwrap_or_else(|e| panic!("write {}: {}", dst.display(), e));
}
