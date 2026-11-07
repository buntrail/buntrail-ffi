fn main() {
    let directory = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let destination = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::create_dir_all(&destination).unwrap();
    cbindgen::Builder::new()
        .with_crate(&directory)
        .with_language(cbindgen::Language::C)
        .with_include_guard("BUNTRAIL_H")
        .with_cpp_compat(true)
        .generate()
        .expect("generate native ABI")
        .write_to_file(destination.join("buntrail.h"));
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/editor.rs");
}
