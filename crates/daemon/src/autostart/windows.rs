use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn task_name() -> String {
    format!("{} Daemon", common::app::NAME)
}

fn schtasks(args: &[&str]) -> Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("schtasks")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("failed to run schtasks")
}

fn access_denied(out: &std::process::Output) -> bool {
    String::from_utf8_lossy(&out.stderr)
        .to_ascii_lowercase()
        .contains("access is denied")
}

// `runas` cannot inherit our stdio across the elevation boundary, so success is
// read from the elevated exit code; a declined UAC consent makes Start-Process
// throw.
fn schtasks_elevated(args: &[&str]) -> Result<()> {
    use std::os::windows::process::CommandExt;
    let list = args
        .iter()
        .map(|a| format!("'{}'", a.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        "$ErrorActionPreference='Stop'; try {{ \
         $p = Start-Process -FilePath schtasks.exe -ArgumentList {list} \
         -Verb RunAs -WindowStyle Hidden -PassThru -Wait; exit $p.ExitCode \
         }} catch {{ exit 1 }}"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .context("failed to launch the elevation helper")?;
    if !out.status.success() {
        bail!("autostart needs administrator approval, which was declined");
    }
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn current_user() -> String {
    let user = std::env::var("USERNAME").unwrap_or_default();
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{user}"),
        _ => user,
    }
}

fn task_xml(exe: &Path, workdir: &Path, user: &str) -> String {
    include_str!("../../assets/autostart_task.xml")
        .replace(
            "@DESCRIPTION@",
            &xml_escape(&format!("{} launcher daemon", common::app::NAME)),
        )
        .replace("@USER@", &xml_escape(user))
        .replace("@COMMAND@", &xml_escape(&exe.display().to_string()))
        .replace("@WORKDIR@", &xml_escape(&workdir.display().to_string()))
}

fn write_utf16(path: &Path, s: &str) -> Result<()> {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in s.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(path, bytes).context("cannot write the autostart task definition")
}

pub(super) fn enable() -> Result<()> {
    let exe = std::env::current_exe().context("cannot resolve daemon executable path")?;
    let workdir = exe.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let xml = task_xml(&exe, &workdir, &current_user());

    let tmp = std::env::temp_dir().join("hestia-autostart.xml");
    write_utf16(&tmp, &xml)?;
    let name = task_name();
    let path = tmp.to_string_lossy().into_owned();
    let args = ["/Create", "/XML", &path, "/TN", &name, "/F"];

    let out = schtasks(&args)?;
    let result = if out.status.success() {
        Ok(())
    } else if access_denied(&out) {
        schtasks_elevated(&args)
    } else {
        let detail = String::from_utf8_lossy(&out.stderr);
        Err(anyhow!(
            "schtasks failed to create the autostart task: {}",
            detail.trim()
        ))
    };
    let _ = std::fs::remove_file(&tmp);
    result
}

pub(super) fn disable() -> Result<()> {
    let name = task_name();
    let args = ["/Delete", "/F", "/TN", &name];
    let out = schtasks(&args)?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("does not exist") || lower.contains("cannot find") {
        return Ok(());
    }
    if access_denied(&out) {
        return schtasks_elevated(&args);
    }
    bail!(
        "schtasks failed to remove the autostart task: {}",
        stderr.trim()
    )
}

pub(super) fn is_enabled() -> bool {
    schtasks(&["/Query", "/TN", &task_name()])
        .map(|o| o.status.success())
        .unwrap_or(false)
}
