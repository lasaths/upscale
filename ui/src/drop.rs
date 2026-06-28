use crate::models::OutputFormat;
use std::path::{Path, PathBuf};

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp"];

pub fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
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
    let aligned = ((max_side + 31) / 32) * 32;
    let cap = match scale {
        4 => 256,
        3 => 384,
        _ => 512,
    };
    aligned.min(cap).max(32)
}

pub fn gpu_id() -> String {
    std::env::var("UPSCALE_GPU").unwrap_or_else(|_| "1".into())
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
}
