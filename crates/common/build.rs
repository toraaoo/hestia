//! Cargo does not track what `option_env!` reads, so a changed build-time
//! channel would otherwise not rebuild the crate that bakes it in — a warm
//! cache would ship a beta build stamped `dev`, pointed at the stable feed.
fn main() {
    println!("cargo:rerun-if-env-changed=HESTIA_CHANNEL");
}
