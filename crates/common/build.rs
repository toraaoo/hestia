use std::path::Path;

// Debug builds anchor Hestia's data directory at <workspace>/.hestia so
// development never populates the real per-user directory; compiled out of
// release. The workspace root is two levels up from this crate.
fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace = Path::new(&manifest)
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let dev_home = workspace.join(".hestia");
    println!("cargo:rustc-env=HESTIA_DEV_HOME={}", dev_home.display());
    println!("cargo:rerun-if-changed=build.rs");
}
