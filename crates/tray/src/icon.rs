//! The tray icon: a transparent 256px raster of the Ember mark.
//!
//! Keep the embedded PNG at this density rather than exporting a panel-sized
//! bitmap. Tray hosts choose their own logical size (and scale factor), and a
//! larger source gives their resampler enough edge detail to avoid stair-step
//! diagonals. Decoding stops at RGBA here; each platform packs it into the
//! shape its own status area wants.

use anyhow::{ensure, Context, Result};

// Generated from assets/icons/ember.svg by scripts/gen-icons.sh — do not edit.
const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn load() -> Result<Raster> {
    let decoder = png::Decoder::new(std::io::Cursor::new(ICON_PNG));
    let mut reader = decoder.read_info().context("read icon header")?;
    let size = reader
        .output_buffer_size()
        .context("icon dimensions overflow")?;
    let mut rgba = vec![0u8; size];
    let info = reader.next_frame(&mut rgba).context("decode icon")?;
    ensure!(
        info.color_type == png::ColorType::Rgba && info.bit_depth == png::BitDepth::Eight,
        "embedded icon must be 8-bit RGBA, got {:?}/{:?}",
        info.color_type,
        info.bit_depth
    );
    rgba.truncate(info.buffer_size());
    Ok(Raster {
        width: info.width,
        height: info.height,
        rgba,
    })
}
