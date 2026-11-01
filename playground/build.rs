fn main() {
    marty::htaccess::write_next_to_binary_from_manifest()
        .expect("write htaccess from [package.metadata.marty]");
    println!("cargo:rerun-if-changed=build.rs");
}
