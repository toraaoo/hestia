use anyhow::{bail, Result};

pub(super) fn enable() -> Result<()> {
    bail!("autostart is not supported on this platform yet")
}

pub(super) fn disable() -> Result<()> {
    bail!("autostart is not supported on this platform yet")
}

pub(super) fn is_enabled() -> bool {
    false
}
