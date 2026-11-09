use std::fmt::Write as _;
use std::path::{Component, Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo = manifest_dir.join("..").join("..");
    embed(
        &repo.join("menupack"),
        &["assets", "pack.png"],
        "assets/minecraft/texts/splashes.txt",
        "menupack_files.rs",
    );
}

fn embed(pack_dir: &Path, top_level: &[&str], required: &str, output: &str) {
    println!("cargo:rerun-if-changed={}", pack_dir.display());

    let mut files = Vec::new();
    collect(pack_dir, pack_dir, top_level, &mut files);
    files.sort();
    assert!(
        files.iter().any(|(name, _)| name == required),
        "{} is missing in {}",
        required,
        pack_dir.display()
    );

    let mut source = String::from(
        "&[
",
    );
    for (name, path) in &files {
        writeln!(
            source,
            "    ({name:?}, include_bytes!({:?})),",
            path.display()
        )
        .expect("write");
    }
    source.push(']');
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    std::fs::write(out_dir.join(output), source).expect("write pack index");
}

fn collect(root: &Path, dir: &Path, top_level: &[&str], files: &mut Vec<(String, PathBuf)>) {
    for entry in std::fs::read_dir(dir).expect("read pack folder") {
        let path = entry.expect("pack entry").path();
        if path.is_dir() {
            collect(root, &path, top_level, files);
            continue;
        }
        let relative = path.strip_prefix(root).expect("inside pack");
        let parts: Vec<&str> = relative
            .components()
            .map(|component| match component {
                Component::Normal(part) => part.to_str().expect("UTF-8 file name"),
                _ => panic!("unexpected path component in {}", relative.display()),
            })
            .collect();
        assert!(
            top_level.contains(&parts[0]),
            "{} is outside {}",
            relative.display(),
            top_level.join(", ")
        );
        files.push((parts.join("/"), path));
    }
}
