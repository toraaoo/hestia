//! Custom entry icons, copied into the hestia data home.
//!
//! A picked image is copied to `<data_home>/icons/<entry-id>.<ext>` so it
//! survives the original file moving; the disk is the registry (no index).
//! The webview loads them over the asset protocol, whose scope is widened to
//! the icons directory at each call — the data home can move at runtime.
//!
//! Entries can instead wear a *generated* icon: a flat PNG composited here
//! from a background (solid color or vertical gradient) and a symbol sprite
//! ship from the frontend, saved as `<entry-id>.png`. The configuration that
//! produced it is kept beside it as an `<entry-id>.json` sidecar so the icon
//! editor can reopen on the current look — the disk is the registry for both.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::bridge::CallError;

const MAX_BYTES: u64 = 10 * 1024 * 1024;
const EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

/// A generated icon's geometry; the sidecar documents the config that made it.
const GENERATED_ICON_SIZE: u32 = 256;
const MAX_ICON_CONFIG_ID_LENGTH: usize = 64;
const MAX_SYMBOL_BYTES: usize = 4 * 1024 * 1024;
const MAX_SYMBOL_DIMENSION: u32 = 4096;

/// The wire shape the editor catalogue and the shell agree on.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IconBackground {
    Color {
        value: String,
    },
    #[serde(rename = "linear-top-down-gradient")]
    LinearTopDownGradient {
        top_color: String,
        bottom_color: String,
    },
}

/// One generated icon: its background plus the symbol sprite id.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct IconConfig {
    pub background: IconBackground,
    pub symbol: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CanvasBackground {
    Color([u8; 3]),
    LinearTopDownGradient {
        top_color: [u8; 3],
        bottom_color: [u8; 3],
    },
}

/// One entry's stored icon; `mtime` doubles as the cache-busting version.
#[derive(Serialize, Clone)]
pub struct IconEntry {
    pub path: String,
    pub mtime: u64,
}

fn icons_dir() -> PathBuf {
    common::paths::data_home(None).join("icons")
}

// Entry ids are slug + hex tag; anything else could escape the icons dir.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn allow_assets(app: &AppHandle, dir: &Path) {
    let _ = app.asset_protocol_scope().allow_directory(dir, false);
}

fn entry_for(path: &Path) -> Option<IconEntry> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(IconEntry {
        path: path.to_string_lossy().into_owned(),
        mtime,
    })
}

fn stored_icons(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            let id = path.file_stem()?.to_str()?.to_string();
            let icon = path.is_file() && has_image_extension(&path);
            icon.then_some((id, path))
        })
        .collect()
}

fn has_image_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

fn remove_stored(dir: &Path, id: &str) {
    for (stored_id, path) in stored_icons(dir) {
        if stored_id == id {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[tauri::command]
pub fn icons_list(app: AppHandle) -> Result<BTreeMap<String, IconEntry>, CallError> {
    let dir = icons_dir();
    allow_assets(&app, &dir);
    Ok(stored_icons(&dir)
        .into_iter()
        .filter_map(|(id, path)| entry_for(&path).map(|entry| (id, entry)))
        .collect())
}

#[tauri::command]
pub fn icon_set(
    app: AppHandle,
    entry_id: String,
    source_path: String,
) -> Result<IconEntry, CallError> {
    if !valid_id(&entry_id) {
        return Err(CallError::other("invalid entry id"));
    }
    let source = PathBuf::from(&source_path);
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|e| EXTENSIONS.contains(&e.as_str()))
        .ok_or_else(|| CallError::other("unsupported image type"))?;
    let size = std::fs::metadata(&source)
        .map_err(|e| CallError::other(e.to_string()))?
        .len();
    if size > MAX_BYTES {
        return Err(CallError::other("image is larger than 10 MB"));
    }

    let dir = icons_dir();
    std::fs::create_dir_all(&dir).map_err(|e| CallError::other(e.to_string()))?;
    remove_stored(&dir, &entry_id);
    let target = dir.join(format!("{entry_id}.{ext}"));
    std::fs::copy(&source, &target).map_err(|e| CallError::other(e.to_string()))?;
    tracing::info!(entry_id, ext, size, "icon set");
    allow_assets(&app, &dir);
    entry_for(&target).ok_or_else(|| CallError::other("cannot read the stored icon"))
}

#[tauri::command]
pub fn icon_remove(entry_id: String) -> Result<(), CallError> {
    if !valid_id(&entry_id) {
        return Err(CallError::other("invalid entry id"));
    }
    tracing::info!(entry_id, "icon removed");
    remove_stored(&icons_dir(), &entry_id);
    Ok(())
}

/// Compose `<entry-id>.png` from a background + symbol and record the config
/// as an `<entry-id>.json` sidecar, replacing whatever icon the entry wore.
#[tauri::command]
pub async fn icon_generate(
    app: AppHandle,
    entry_id: String,
    config: IconConfig,
    symbol_bytes: Vec<u8>,
) -> Result<IconEntry, CallError> {
    if !valid_id(&entry_id) {
        return Err(CallError::other("invalid entry id"));
    }
    let background = validate_icon_config(&config)?;
    let png = tokio::task::spawn_blocking(move || render_generated_icon(background, &symbol_bytes))
        .await
        .map_err(|_| CallError::other("icon render worker panicked"))??;

    let dir = icons_dir();
    std::fs::create_dir_all(&dir).map_err(|e| CallError::other(e.to_string()))?;
    remove_stored(&dir, &entry_id);
    let target = dir.join(format!("{entry_id}.png"));
    std::fs::write(&target, &png).map_err(|e| CallError::other(e.to_string()))?;
    let sidecar = dir.join(format!("{entry_id}.json"));
    std::fs::write(
        &sidecar,
        serde_json::to_vec(&config).expect("icon config is serializable"),
    )
    .map_err(|e| CallError::other(e.to_string()))?;
    tracing::info!(entry_id, byte_len = png.len(), "generated icon set");
    allow_assets(&app, &dir);
    entry_for(&target).ok_or_else(|| CallError::other("cannot read the stored icon"))
}

/// The stored generation config, or `null` when the icon was picked from a
/// file instead — the sidecar's only reader.
#[tauri::command]
pub fn icon_config(entry_id: String) -> Result<Option<IconConfig>, CallError> {
    if !valid_id(&entry_id) {
        return Err(CallError::other("invalid entry id"));
    }
    let path = icons_dir().join(format!("{entry_id}.json"));
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| CallError::other(e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(CallError::other(e.to_string())),
    }
}

fn validate_icon_config(config: &IconConfig) -> Result<CanvasBackground, CallError> {
    let background = match &config.background {
        IconBackground::Color { value } => CanvasBackground::Color(parse_background_color(value)?),
        IconBackground::LinearTopDownGradient {
            top_color,
            bottom_color,
        } => CanvasBackground::LinearTopDownGradient {
            top_color: parse_background_color(top_color)?,
            bottom_color: parse_background_color(bottom_color)?,
        },
    };
    validate_symbol_id(&config.symbol)?;
    Ok(background)
}

fn parse_background_color(value: &str) -> Result<[u8; 3], CallError> {
    if value.len() != 7 || !value.starts_with('#') {
        return Err(CallError::other(
            "icon background must be a hexadecimal color",
        ));
    }
    let color = u32::from_str_radix(&value[1..], 16)
        .map_err(|_| CallError::other("icon background must be a hexadecimal color"))?;
    Ok([
        ((color >> 16) & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        (color & 0xff) as u8,
    ])
}

fn validate_symbol_id(value: &str) -> Result<(), CallError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_ICON_CONFIG_ID_LENGTH
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    valid
        .then_some(())
        .ok_or_else(|| CallError::other("invalid symbol id"))
}

/// Paints the 256×256 icon: the background across the canvas, the symbol
/// stretched to fill it, then every pixel forced opaque (a launcher icon has
/// no place for transparency).
fn render_generated_icon(
    background: CanvasBackground,
    symbol_bytes: &[u8],
) -> Result<Vec<u8>, CallError> {
    if symbol_bytes.is_empty() || symbol_bytes.len() > MAX_SYMBOL_BYTES {
        return Err(CallError::other(
            "icon symbol must be a PNG smaller than 4 MiB",
        ));
    }

    let reader = ImageReader::with_format(Cursor::new(symbol_bytes), ImageFormat::Png);
    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| CallError::other(format!("invalid icon symbol: {e}")))?;
    if width == 0 || height == 0 || width > MAX_SYMBOL_DIMENSION || height > MAX_SYMBOL_DIMENSION {
        return Err(CallError::other(format!(
            "icon symbol dimensions must be between 1 and {MAX_SYMBOL_DIMENSION} pixels"
        )));
    }

    let symbol = image::load_from_memory_with_format(symbol_bytes, ImageFormat::Png)
        .map_err(|e| CallError::other(format!("invalid icon symbol: {e}")))?
        .resize_exact(
            GENERATED_ICON_SIZE,
            GENERATED_ICON_SIZE,
            FilterType::Lanczos3,
        )
        .to_rgba8();
    let mut icon = match background {
        CanvasBackground::Color(color) => RgbaImage::from_pixel(
            GENERATED_ICON_SIZE,
            GENERATED_ICON_SIZE,
            Rgba([color[0], color[1], color[2], 255]),
        ),
        CanvasBackground::LinearTopDownGradient {
            top_color,
            bottom_color,
        } => RgbaImage::from_fn(GENERATED_ICON_SIZE, GENERATED_ICON_SIZE, |_, y| {
            let interpolate = |top: u8, bottom: u8| {
                let distance = i32::from(bottom) - i32::from(top);
                (i32::from(top) + distance * y as i32 / (GENERATED_ICON_SIZE - 1) as i32) as u8
            };
            Rgba([
                interpolate(top_color[0], bottom_color[0]),
                interpolate(top_color[1], bottom_color[1]),
                interpolate(top_color[2], bottom_color[2]),
                255,
            ])
        }),
    };
    image::imageops::overlay(&mut icon, &symbol, 0, 0);
    for pixel in icon.pixels_mut() {
        pixel[3] = 255;
    }

    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(icon)
        .write_to(&mut encoded, ImageFormat::Png)
        .map_err(|e| CallError::other(format!("invalid icon symbol: {e}")))?;
    Ok(encoded.into_inner())
}

#[cfg(test)]
mod tests {
    use super::{
        render_generated_icon, validate_icon_config, CanvasBackground, IconBackground, IconConfig,
        GENERATED_ICON_SIZE,
    };
    use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

    fn png(pixel: Rgba<u8>) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(16, 16, pixel))
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    fn center_pixel(bytes: &[u8]) -> Rgba<u8> {
        let icon = image::load_from_memory_with_format(bytes, ImageFormat::Png).unwrap();
        let (width, height) = icon.dimensions();
        *icon.to_rgba8().get_pixel(width / 2, height / 2)
    }

    #[test]
    fn generated_icon_renders_background_and_symbol() {
        let bytes = render_generated_icon(
            CanvasBackground::Color([10, 20, 30]),
            &png(Rgba([200, 100, 50, 128])),
        )
        .unwrap();
        let icon = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
            .unwrap()
            .to_rgba8();

        assert_eq!(
            icon.dimensions(),
            (GENERATED_ICON_SIZE, GENERATED_ICON_SIZE)
        );
        assert_eq!(center_pixel(&bytes), Rgba([105, 60, 40, 255]));
    }

    #[test]
    fn generated_icon_renders_linear_top_down_gradient() {
        let bytes = render_generated_icon(
            CanvasBackground::LinearTopDownGradient {
                top_color: [10, 20, 30],
                bottom_color: [110, 120, 130],
            },
            &png(Rgba([0, 0, 0, 0])),
        )
        .unwrap();
        let icon = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
            .unwrap()
            .to_rgba8();

        assert_eq!(icon.get_pixel(0, 0), &Rgba([10, 20, 30, 255]));
        assert_eq!(
            icon.get_pixel(0, GENERATED_ICON_SIZE - 1),
            &Rgba([110, 120, 130, 255])
        );
    }

    #[test]
    fn generated_icon_rejects_invalid_symbol_data() {
        assert!(render_generated_icon(CanvasBackground::Color([0, 0, 0]), b"not a png").is_err());
    }

    #[test]
    fn icon_config_validates_color_and_symbol_id() {
        assert_eq!(
            validate_icon_config(&IconConfig {
                background: IconBackground::Color {
                    value: "#c78aff".to_string(),
                },
                symbol: "dusk_block".to_string(),
            })
            .unwrap(),
            CanvasBackground::Color([199, 138, 255])
        );
        assert!(validate_icon_config(&IconConfig {
            background: IconBackground::Color {
                value: "purple".to_string(),
            },
            symbol: "dusk_block".to_string(),
        })
        .is_err());
        assert!(validate_icon_config(&IconConfig {
            background: IconBackground::Color {
                value: "#c78aff".to_string(),
            },
            symbol: "dusk-block".to_string(),
        })
        .is_err());
    }
}
