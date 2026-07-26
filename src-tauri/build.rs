use std::path::{Path, PathBuf};

const WOFF2_SOURCES: &[&str] = &[
    "src/table_tags.cc",
    "src/variable_length.cc",
    "src/woff2_common.cc",
    "src/font.cc",
    "src/glyph.cc",
    "src/normalize.cc",
    "src/transform.cc",
    "src/woff2_enc.cc",
];

const BROTLI_ENCODER_SOURCES: &[&str] = &[
    "c/enc/backward_references.c",
    "c/enc/backward_references_hq.c",
    "c/enc/bit_cost.c",
    "c/enc/block_splitter.c",
    "c/enc/brotli_bit_stream.c",
    "c/enc/cluster.c",
    "c/enc/compress_fragment.c",
    "c/enc/compress_fragment_two_pass.c",
    "c/enc/dictionary_hash.c",
    "c/enc/encode.c",
    "c/enc/encoder_dict.c",
    "c/enc/entropy_encode.c",
    "c/enc/histogram.c",
    "c/enc/literal_cost.c",
    "c/enc/memory.c",
    "c/enc/metablock.c",
    "c/enc/static_dict.c",
    "c/enc/utf8_util.c",
];

const BROTLI_COMMON_SOURCES: &[&str] = &["c/common/dictionary.c", "c/common/transform.c"];

fn main() {
    build_google_woff2();
    tauri_build::build();
}

fn build_google_woff2() {
    let manifest_dir = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set"),
    );
    let upstream = manifest_dir.join("../vendor/woff2");
    let brotli = upstream.join("brotli");

    require_checkout(&upstream.join("include/woff2/encode.h"));
    require_checkout(&brotli.join("c/include/brotli/encode.h"));

    let mut woff2 = cc::Build::new();
    woff2
        .cpp(true)
        .include(upstream.join("include"))
        .include(upstream.join("src"))
        .include(brotli.join("c/include"))
        .file(manifest_dir.join("native/woff2_wrapper.cc"))
        .define("__STDC_FORMAT_MACROS", None)
        .flag_if_supported("-std=c++11")
        .flag_if_supported("/std:c++14")
        .flag_if_supported("/EHsc");
    for source in WOFF2_SOURCES {
        woff2.file(upstream.join(source));
    }
    // Keep this archive before its Brotli dependencies in the linker input.
    woff2.compile("ttf2woff2_google_woff2");

    let mut encoder = cc::Build::new();
    encoder.include(brotli.join("c/include"));
    for source in BROTLI_ENCODER_SOURCES {
        encoder.file(brotli.join(source));
    }
    encoder.compile("ttf2woff2_brotli_encoder");

    let mut common = cc::Build::new();
    common.include(brotli.join("c/include"));
    for source in BROTLI_COMMON_SOURCES {
        common.file(brotli.join(source));
    }
    common.compile("ttf2woff2_brotli_common");

    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("native/woff2_wrapper.cc").display()
    );
    println!("cargo:rerun-if-changed={}", upstream.display());
}

fn require_checkout(path: &Path) {
    if !path.is_file() {
        panic!(
            "Google WOFF2 sources are missing at {}. Run `git submodule update --init --recursive`.",
            path.display()
        );
    }
}
