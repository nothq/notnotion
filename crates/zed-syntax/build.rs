const BUNDLED_NATIVE_LIBRARIES: &[&str] = &[
    "tree-sitter",
    "tree-sitter-json",
    "tree-sitter-markdown",
    "tree-sitter-python",
    "tree-sitter-rust",
    "tree-sitter-typescript",
];

fn main() {
    for library in BUNDLED_NATIVE_LIBRARIES {
        println!("cargo:rustc-link-lib=static:+bundle={library}");
    }
}
