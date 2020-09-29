extern crate cbindgen;
extern crate bindgen as rsbindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let math_neon_out_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("math_neon.rs");

    // we consume math_neon on arm. generate rs files here with bindgen
    // do not link it. only final exe links
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=support/math_neon.h");

    // generate otters API
    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_namespace("otters")
        .with_include_guard("OTTERS_RT_H_")
        .generate()
        .unwrap()
        .write_to_file("gen/otters_rt.h");

    rsbindgen::Builder::default()
        .header("support/math_neon.h")
        .parse_callbacks(Box::new(rsbindgen::CargoCallbacks))
        .generate()
        .unwrap()
        .write_to_file(math_neon_out_dir)
        .unwrap();
}