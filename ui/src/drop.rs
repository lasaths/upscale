use crate::models::OutputFormat;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp"];

pub fn is_image(path: &Path) -> bool {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
    {
        return true;
    }
    // Browser drops sometimes omit/mangle the extension — sniff magic bytes.
    path.is_file() && image::image_dimensions(path).is_ok()
}

pub fn output_path_for(input: &Path, format: OutputFormat) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".into());
    input.with_file_name(format!("{stem}_upscaled.{}", format.ext()))
}

pub fn collect_paths(path: &Path) -> Vec<PathBuf> {
    if path.is_dir() {
        std::fs::read_dir(path)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file() && is_image(p))
            .collect()
    } else if path.is_file() && is_image(path) {
        vec![path.to_path_buf()]
    } else {
        vec![]
    }
}

pub fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Tile size for NCNN. Smaller at higher scale to avoid VRAM OOM on laptop GPUs.
/// Always pair with single-GPU (`-g`) and `-j 1:1:1` to avoid scrambled stitching.
pub fn tile_size_for(path: &Path, scale: u8) -> u32 {
    let Ok(img) = image::open(path) else {
        return 256;
    };
    let max_side = img.width().max(img.height());
    let aligned = max_side.div_ceil(32) * 32;
    let cap = match scale {
        4 => 256,
        3 => 384,
        _ => 512,
    };
    aligned.min(cap).max(32)
}

pub fn gpu_id() -> String {
    std::env::var("UPSCALE_GPU").unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "0".into()
        } else {
            "1".into()
        }
    })
}

fn inbox_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("loku-inbox");
    std::fs::create_dir_all(&dir).map_err(|e| format!("[ERROR: inbox] {e}"))?;
    Ok(dir)
}

fn unique_inbox_path(name: &str) -> Result<PathBuf, String> {
    let dir = inbox_dir()?;
    let safe = sanitize_filename(name);
    let candidate = dir.join(&safe);
    if !candidate.exists() {
        return Ok(candidate);
    }
    let stem = Path::new(&safe)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".into());
    let ext = Path::new(&safe)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    for i in 1..10_000 {
        let p = dir.join(format!("{stem}-{i}.{ext}"));
        if !p.exists() {
            return Ok(p);
        }
    }
    Err("[ERROR: inbox full]".into())
}

fn sanitize_filename(name: &str) -> String {
    let base = Path::new(name)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "image.png".into());
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned.starts_with('.') {
        "image.png".into()
    } else if Path::new(&cleaned).extension().is_none() {
        format!("{cleaned}.png")
    } else {
        cleaned
    }
}

/// Persist in-memory image bytes (browser drag) into the inbox and return the path.
pub fn ingest_bytes(bytes: &[u8], name: &str) -> Result<PathBuf, String> {
    image::load_from_memory(bytes).map_err(|e| format!("[ERROR: not an image] {e}"))?;
    let path = unique_inbox_path(name)?;
    std::fs::write(&path, bytes).map_err(|e| format!("[ERROR: write] {e}"))?;
    Ok(path)
}

/// Read an image from the system clipboard (Copy Image → Ctrl/Cmd+V).
pub fn paste_clipboard_image() -> Result<PathBuf, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("[ERROR: clipboard] {e}"))?;
    let img = clipboard
        .get_image()
        .map_err(|e| format!("[ERROR: no image on clipboard] {e}"))?;
    let rgba = image::RgbaImage::from_raw(
        img.width as u32,
        img.height as u32,
        img.bytes.into_owned(),
    )
    .ok_or_else(|| "[ERROR: bad clipboard image]".to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = unique_inbox_path(&format!("paste-{stamp}.png"))?;
    rgba.save(&path)
        .map_err(|e| format!("[ERROR: write clipboard image] {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::OutputFormat;
    use std::path::Path;

    #[test]
    fn output_path_suffix() {
        let input = Path::new(r"C:\photos\vacation.jpg");
        let out = output_path_for(input, OutputFormat::Png);
        assert_eq!(out, Path::new(r"C:\photos\vacation_upscaled.png"));
    }

    #[test]
    fn rejects_non_image() {
        assert!(collect_paths(Path::new("readme.txt")).is_empty());
    }

    #[test]
    fn tile_size_respects_scale_cap() {
        assert_eq!(tile_size_for(Path::new("missing.png"), 4), 256);
        assert_eq!(tile_size_for(Path::new("missing.png"), 2), 256);
    }

    #[test]
    fn ingest_bytes_writes_png() {
        let mut png = Vec::new();
        image::RgbImage::from_pixel(2, 2, image::Rgb([1, 2, 3]))
            .write_to(
                &mut std::io::Cursor::new(&mut png),
                image::ImageFormat::Png,
            )
            .unwrap();
        let path = ingest_bytes(&png, "from-web.png").unwrap();
        assert!(path.is_file());
        assert!(is_image(&path));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn sanitize_keeps_extension() {
        assert_eq!(sanitize_filename("photo.webp"), "photo.webp");
        assert_eq!(sanitize_filename("../../x.png"), "x.png");
    }
}
