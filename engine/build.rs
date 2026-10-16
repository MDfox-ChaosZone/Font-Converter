use std::{fs, path::Path};

fn sources(build: &mut cc::Build, dir: &Path) {
    let mut files: Vec<_> = fs::read_dir(dir)
        .expect("Initialize vendor submodules")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "c"))
        .collect();
    files.sort();
    for file in files {
        build.file(file);
    }
}

fn main() {
    let vendor = Path::new("../vendor");
    let woff = vendor.join("woff2");
    let brotli = vendor.join("brotli");
    assert!(
        woff.join("include/woff2/encode.h").exists()
            && brotli.join("c/include/brotli/encode.h").exists(),
        "Run git submodule update --init --recursive"
    );
    let mut codec = cc::Build::new();
    codec
        .cpp(true)
        .include(woff.join("include"))
        .include(woff.join("src"))
        .include(brotli.join("c/include"))
        .file("native/codec.cc")
        .flag_if_supported("-std=c++11")
        .flag_if_supported("/std:c++14")
        .flag_if_supported("/EHsc");
    for name in [
        "table_tags",
        "variable_length",
        "woff2_common",
        "font",
        "glyph",
        "normalize",
        "transform",
        "woff2_enc",
        "woff2_dec",
        "woff2_out",
    ] {
        codec.file(woff.join(format!("src/{name}.cc")));
    }
    codec.compile("fc_woff2");
    for part in ["enc", "dec", "common"] {
        let mut build = cc::Build::new();
        build.include(brotli.join("c/include"));
        sources(&mut build, &brotli.join("c").join(part));
        build.compile(&format!("fc_brotli_{part}"));
    }
    println!("cargo:rerun-if-changed=native/codec.cc");
    println!("cargo:rerun-if-changed=../vendor/woff2");
    println!("cargo:rerun-if-changed=../vendor/brotli");
}
