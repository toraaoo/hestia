//! The tray icon: a transparent, high-resolution crop of the Ember mark.

use anyhow::{ensure, Context, Result};
use tray_icon::Icon;

const ICON_PNG: &[u8] = include_bytes!("../assets/ember-block.png");

pub fn load() -> Result<Icon> {
    let decoder = png::Decoder::new(std::io::Cursor::new(ICON_PNG));
    let mut reader = decoder.read_info().context("read icon header")?;
    let size = reader
        .output_buffer_size()
        .context("icon dimensions overflow")?;
    let mut buf = vec![0u8; size];
    let info = reader.next_frame(&mut buf).context("decode icon")?;
    ensure!(
        info.color_type == png::ColorType::Rgba && info.bit_depth == png::BitDepth::Eight,
        "embedded icon must be 8-bit RGBA, got {:?}/{:?}",
        info.color_type,
        info.bit_depth
    );
    buf.truncate(info.buffer_size());
    Icon::from_rgba(buf, info.width, info.height).context("build tray icon")
}
