//! The tray icon: a transparent 256px raster of the Ember mark.
//!
//! Keep the embedded PNG at this density rather than exporting a panel-sized
//! bitmap. Tray hosts choose their own logical size (and scale factor), and a
//! larger source gives their resampler enough edge detail to avoid stair-step
//! diagonals.

use anyhow::{ensure, Context, Result};
use tray_icon::Icon;

// Rasterized from `icon.svg` at 256x256 with librsvg.
const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

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
