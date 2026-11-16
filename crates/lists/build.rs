use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let lists_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../lists");
    println!("cargo::rerun-if-changed={}", lists_dir.display());
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));

    let mut ids: Vec<String> = fs::read_dir(&lists_dir)
        .expect("lists/ exists at the workspace root")
        .map(|entry| entry.expect("lists/ is readable").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .map(|path| path.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    ids.sort();

    let mut code = format!(
        "pub(crate) const LIST_COUNT: usize = {};\n\
         pub(crate) static BLOBS: [(&str, &[u8]); LIST_COUNT] = [\n",
        ids.len()
    );
    for id in &ids {
        assert!(
            id.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
            "list file name {id:?} must be [a-z0-9-]"
        );
        let text = fs::read(lists_dir.join(format!("{id}.txt"))).expect("list file is readable");
        let blob = out_dir.join(format!("{id}.lz4"));
        fs::write(&blob, lz4_flex::compress_prepend_size(&text)).expect("OUT_DIR is writable");
        writeln!(
            code,
            "    ({id:?}, include_bytes!({:?})),",
            blob.display().to_string()
        )
        .unwrap();
    }
    code.push_str("];\n");
    fs::write(out_dir.join("blobs.rs"), code).expect("OUT_DIR is writable");
}
