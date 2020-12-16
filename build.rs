extern crate bindgen;

use std::env;
use std::path::PathBuf;

#[cfg(feature = "docs-rs")]
fn main() {}

#[cfg(not(feature = "docs-rs"))]
fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-search={}", r"openjpeg\install\lib");
    println!("cargo:rustc-link-lib=openjp2");

    let mut builder = bindgen::Builder::default();

    builder = builder.clang_arg(format!("-I{}", r"openjpeg\install\include\openjpeg-2.3"));

    let bindings = builder
        .header_contents("wrapper.h", "#include \"openjpeg.h\"")
        .clang_arg("-fno-inline-functions")
        .derive_debug(true)
        .impl_debug(true)
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .rustfmt_bindings(true)
        .generate()
        .unwrap();

    // bindings.write_to_file("src/ffi.ref.rs").unwrap();

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap();
}
