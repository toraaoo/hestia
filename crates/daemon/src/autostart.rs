//! Login-time autostart registration, driven by the reserved `autostart` config
//! key. Registers the running daemon's own executable so the registration
//! survives the binary being moved.

use anyhow::{Context, Result};

#[cfg(target_os = "linux")]
mod backend {
    use std::path::PathBuf;

    use anyhow::{Context, Result};

    const UNIT_NAME: &str = "hestiad.service";

    fn systemd_user_dir() -> Result<PathBuf> {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return Ok(PathBuf::from(xdg).join("systemd").join("user"));
            }
        }
        let home =
            std::env::var_os("HOME").context("cannot resolve systemd user dir: HOME is unset")?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("systemd")
            .join("user"))
    }

    fn unit_contents() -> Result<String> {
        let exe = std::env::current_exe().context("cannot resolve daemon executable path")?;
        Ok(format!(
            "[Unit]\nDescription={name} launcher daemon\nAfter=default.target\n\n\
             [Service]\nType=simple\nExecStart={exe} serve\nRestart=on-failure\n\n\
             [Install]\nWantedBy=default.target\n",
            name = common::app::NAME,
            exe = exe.display(),
        ))
    }

    fn reload() {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    pub fn enable() -> Result<()> {
        let dir = systemd_user_dir()?;
        std::fs::create_dir_all(&dir)?;
        let unit = dir.join(UNIT_NAME);
        std::fs::write(&unit, unit_contents()?)?;

        let wants = dir.join("default.target.wants");
        std::fs::create_dir_all(&wants)?;
        let link = wants.join(UNIT_NAME);
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&unit, &link).context("cannot enable autostart")?;
        reload();
        Ok(())
    }

    pub fn disable() -> Result<()> {
        let dir = systemd_user_dir()?;
        let _ = std::fs::remove_file(dir.join("default.target.wants").join(UNIT_NAME));
        let _ = std::fs::remove_file(dir.join(UNIT_NAME));
        reload();
        Ok(())
    }

    pub fn is_enabled() -> bool {
        systemd_user_dir()
            .map(|dir| dir.join("default.target.wants").join(UNIT_NAME).exists())
            .unwrap_or(false)
    }
}

#[cfg(windows)]
mod backend {
    use anyhow::{bail, Context, Result};

    // A logon Scheduled Task, matching the C++ backend. Task Scheduler owns the
    // persistence, queried back via is_enabled().
    fn task_name() -> String {
        format!("{} Daemon", common::app::NAME)
    }

    fn schtasks(args: &[&str]) -> Result<std::process::Output> {
        use std::os::windows::process::CommandExt;
        // The daemon usually runs without a console; a console child would
        // otherwise flash a window on every invocation.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("schtasks")
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .context("failed to run schtasks")
    }

    pub fn enable() -> Result<()> {
        let exe = std::env::current_exe().context("cannot resolve daemon executable path")?;
        // /TR carries the command as one token: the exe path is quoted (it may
        // contain spaces) and its inner quotes are escaped for the tokenizer.
        let tr = format!("\\\"{}\\\" serve", exe.display());
        let out = schtasks(&[
            "/Create",
            "/F",
            "/SC",
            "ONLOGON",
            "/TN",
            &task_name(),
            "/TR",
            &tr,
        ])?;
        if !out.status.success() {
            bail!("schtasks failed to create the autostart task");
        }
        Ok(())
    }

    pub fn disable() -> Result<()> {
        let _ = schtasks(&["/Delete", "/F", "/TN", &task_name()]);
        Ok(())
    }

    pub fn is_enabled() -> bool {
        schtasks(&["/Query", "/TN", &task_name()])
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

// TODO(macos): implement a LaunchAgent backend (write the ~/Library/LaunchAgents
// plist keyed by common::app::ID with RunAtLoad, best-effort launchctl
// load/unload), mirroring the C++ LaunchAgentAutostart. Deferred for now.
#[cfg(not(any(target_os = "linux", windows)))]
mod backend {
    use anyhow::{bail, Result};

    pub fn enable() -> Result<()> {
        bail!("autostart is not supported on this platform yet")
    }
    pub fn disable() -> Result<()> {
        bail!("autostart is not supported on this platform yet")
    }
    pub fn is_enabled() -> bool {
        false
    }
}

pub fn is_enabled() -> bool {
    backend::is_enabled()
}

pub fn set(enabled: bool) -> Result<()> {
    let result = if enabled {
        backend::enable().context("failed to enable autostart")
    } else {
        backend::disable().context("failed to disable autostart")
    };
    if result.is_ok() {
        tracing::info!(enabled, "autostart registration changed");
    }
    result
}
