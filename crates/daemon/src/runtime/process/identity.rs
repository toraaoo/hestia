//! Platform probes for process identity and termination. The identity token is
//! a start-time value that is stable for one process and different for any
//! later process reusing its pid — the guard that re-adoption never grabs a
//! stranger.

#[cfg(target_os = "linux")]
pub fn identify(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // Fields after the last ')' start at field 3 (state); starttime is field 22.
    let rest = stat.rsplit_once(')')?.1;
    let mut fields = rest.split_whitespace();
    if fields.next()? == "Z" {
        return None;
    }
    fields.nth(18)?.parse().ok()
}

#[cfg(target_os = "macos")]
pub fn identify(pid: u32) -> Option<u64> {
    let mut mib = [
        libc::CTL_KERN,
        libc::KERN_PROC,
        libc::KERN_PROC_PID,
        pid as libc::c_int,
    ];
    let mut info = std::mem::MaybeUninit::<libc::kinfo_proc>::uninit();
    let mut size = std::mem::size_of::<libc::kinfo_proc>();
    let rc = unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as libc::c_uint,
            info.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 || size < std::mem::size_of::<libc::kinfo_proc>() {
        return None;
    }
    let info = unsafe { info.assume_init() };
    if info.kp_proc.p_pid != pid as libc::pid_t {
        return None;
    }
    let tv = info.kp_proc.p_starttime;
    Some((tv.tv_sec as u64) * 1_000_000 + tv.tv_usec as u64)
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
pub fn identify(pid: u32) -> Option<u64> {
    // No start-time oracle wired for this platform: existence only, which
    // cannot rule out pid reuse.
    (unsafe { libc::kill(pid as libc::pid_t, 0) } == 0).then_some(1)
}

#[cfg(windows)]
pub fn identify(pid: u32) -> Option<u64> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut exit_code = 0u32;
        let running =
            GetExitCodeProcess(handle, &mut exit_code) != 0 && exit_code == STILL_ACTIVE as u32;
        let zero = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut creation = zero;
        let (mut exit, mut kernel, mut user) = (zero, zero, zero);
        let timed = GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) != 0;
        CloseHandle(handle);
        if !running || !timed {
            return None;
        }
        Some(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
    }
}

pub fn is_same(pid: u32, token: u64) -> bool {
    token != 0 && identify(pid) == Some(token)
}

// pid 0 would address the daemon's own process group.
#[cfg(unix)]
pub fn request_stop(pid: u32) {
    if pid != 0 {
        unsafe {
            libc::kill(pid as libc::pid_t, libc::SIGTERM);
        }
    }
}

#[cfg(unix)]
pub fn kill(pid: u32) {
    if pid != 0 {
        unsafe {
            libc::kill(pid as libc::pid_t, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
pub fn request_stop(pid: u32) {
    terminate(pid);
}

#[cfg(windows)]
pub fn kill(pid: u32) {
    terminate(pid);
}

#[cfg(windows)]
fn terminate(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !handle.is_null() {
            TerminateProcess(handle, 1);
            CloseHandle(handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_the_current_process() {
        let token = identify(std::process::id()).expect("own process has an identity");
        assert!(is_same(std::process::id(), token));
    }

    #[test]
    fn rejects_a_wrong_token() {
        let token = identify(std::process::id()).unwrap();
        assert!(!is_same(std::process::id(), token.wrapping_add(1)));
        assert!(!is_same(std::process::id(), 0));
    }

    #[test]
    fn missing_pid_has_no_identity() {
        assert_eq!(identify(u32::MAX - 1), None);
    }
}
