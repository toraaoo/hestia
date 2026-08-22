//! Whether this session can host a tray icon at all.
//!
//! On Linux `tray-icon` reaches the status area through libappindicator,
//! which is dlopened at first use and **panics** when no variant is present —
//! a hard crash on desktops that ship none (GNOME without the extension,
//! SteamOS). The daemon spawns the tray on every serve, so that panic became a
//! crash report on every start. Probing the same names up front turns it into
//! a clean exit.

#[cfg(target_os = "linux")]
const LIBRARIES: [&str; 4] = [
    "libayatana-appindicator3.so.1",
    "libappindicator3.so.1",
    "libayatana-appindicator3.so",
    "libappindicator3.so",
];

#[cfg(target_os = "linux")]
pub fn available() -> bool {
    LIBRARIES.iter().any(|name| loads(name))
}

#[cfg(not(target_os = "linux"))]
pub fn available() -> bool {
    true
}

#[cfg(target_os = "linux")]
fn loads(name: &str) -> bool {
    let Ok(name) = std::ffi::CString::new(name) else {
        return false;
    };
    // SAFETY: `name` is a valid NUL-terminated string. The handle is left open
    // on purpose — libappindicator registers GObject types on load, and
    // unloading a library that has done so is not safe. `tray-icon` reopens
    // the same name later and the loader hands back this refcounted handle.
    !unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_LAZY) }.is_null()
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn a_missing_library_does_not_load() {
        assert!(!super::loads("libhestia-does-not-exist.so.1"));
    }

    #[test]
    fn an_embedded_nul_does_not_load() {
        assert!(!super::loads("lib\0nul.so"));
    }
}
