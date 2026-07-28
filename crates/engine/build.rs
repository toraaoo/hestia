//! Cargo does not track what `option_env!` reads, so a changed build-time
//! CurseForge key would otherwise not rebuild the crate that bakes it in.
fn main() {
    println!("cargo:rerun-if-env-changed=HESTIA_CURSEFORGE_API_KEY");
}
