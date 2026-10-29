//! Writes combined mounts from each example’s `[package.metadata.marty]` to
//! `OUT_DIR/.htaccess` and `examples/apache-combined-vhost.htaccess`.

fn main() {
    let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let examples = [
        "01_basic",
        "02_routing",
        "03_session",
        "04_cookies",
        "05_sqlite",
        "07_mysql",
    ];
    let owned: Vec<_> = examples
        .iter()
        .map(|dir| {
            let path = manifest.join(format!("../{dir}/Cargo.toml"));
            println!("cargo:rerun-if-changed={}", path.display());
            marty::htaccess::owned_mount_from_cargo_toml(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        })
        .collect();
    let mounts: Vec<_> = owned.iter().map(|m| m.as_mount()).collect();

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    marty::htaccess::write(out.join(".htaccess"), &mounts).expect("write OUT_DIR/.htaccess");
    marty::htaccess::write(manifest.join("../apache-combined-vhost.htaccess"), &mounts)
        .expect("write apache-combined-vhost.htaccess");

    println!("cargo:rerun-if-changed=build.rs");
}
