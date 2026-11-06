use crate::models::OutputFormat;
use image::{DynamicImage, Rgb, RgbImage};
use ndarray::Array4;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::{Tensor, ValueType};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Dynamic-shape models: larger tiles keep DirectML busier (128 underfills modern GPUs).
const DEFAULT_TILE: u32 = 256;
/// Input-space overlap between tiles; feathered so seams don't show.
const DEFAULT_OVERLAP: u32 = 32;
const PROBE_SIZE: u32 = 64;

pub fn discover_models(dir: &Path) -> Vec<PathBuf> {
    let mut models = Vec::new();
    let scan_root = dir.join("onnx");
    if !scan_root.is_dir() {
        return models;
    }
    collect_onnx(&scan_root, &mut models);
    models.sort();
    models
}

fn collect_onnx(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_onnx(&path, out);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("onnx")) {
            out.push(path);
        }
    }
}

pub fn model_label(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub fn model_description(path: &Path) -> &'static str {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.contains("nomos") || name.contains("dat") {
        "ONNX — Nomos/DAT, archviz · CGI · natural HD detail"
    } else if name.contains("ultrasharp") {
        "ONNX — UltraSharp, strong general-purpose 4×"
    } else if name.contains("purephoto") || name.contains("realplskr") || name.contains("realplksr") {
        "ONNX — PurePhoto, natural photos and portraits"
    } else if name.contains("anime") {
        "ONNX — anime / illustration model"
    } else if name.contains("hat") {
        "ONNX — HAT transformer, high-quality photos (FP32)"
    } else if name.contains("swin") {
        "ONNX — SwinIR family, faithful restoration"
    } else if name.contains("realesr") || name.contains("esrgan") {
        "ONNX — Real-ESRGAN class, general-purpose 4×"
    } else {
        "ONNX — tiled GPU/CPU inference via ONNX Runtime"
    }
}

struct UpscaleSession {
    session: Session,
    input_name: String,
    output_name: String,
    scale: u32,
    tile_size: u32,
}

fn configured_tile() -> u32 {
    std::env::var("UPSCALE_ONNX_TILE")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| (64..=1024).contains(&n))
        .unwrap_or(DEFAULT_TILE)
}

fn configured_overlap(tile: u32) -> u32 {
    let requested = std::env::var("UPSCALE_ONNX_OVERLAP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_OVERLAP);
    // Keep at least half the tile as stride so we don't explode tile count.
    requested.min(tile / 2)
}

fn tile_starts(len: u32, tile: u32, stride: u32) -> Vec<u32> {
    if len == 0 {
        return Vec::new();
    }
    if len <= tile {
        return vec![0];
    }
    let mut starts = Vec::new();
    let mut x = 0u32;
    while x + tile < len {
        starts.push(x);
        x = x.saturating_add(stride);
    }
    let last = len - tile;
    if starts.last().copied() != Some(last) {
        starts.push(last);
    }
    starts
}

fn blend_weight(x: u32, y: u32, w: u32, h: u32, overlap: u32) -> f32 {
    if overlap == 0 || w == 0 || h == 0 {
        return 1.0;
    }
    let ox = overlap.min(w / 2).max(1);
    let oy = overlap.min(h / 2).max(1);
    let wx = if x < ox {
        (x + 1) as f32 / ox as f32
    } else if x >= w.saturating_sub(ox) {
        (w - x) as f32 / ox as f32
    } else {
        1.0
    };
    let wy = if y < oy {
        (y + 1) as f32 / oy as f32
    } else if y >= h.saturating_sub(oy) {
        (h - y) as f32 / oy as f32
    } else {
        1.0
    };
    (wx * wy).clamp(0.0, 1.0)
}

fn input_tile_size(session: &Session) -> u32 {
    let Some(input) = session.inputs().first() else {
        return configured_tile();
    };
    let ValueType::Tensor { shape, .. } = input.dtype() else {
        return configured_tile();
    };
    let dims: Vec<i64> = shape.iter().copied().collect();
    if dims.len() == 4 {
        let h = dims[2];
        let w = dims[3];
        // Fixed spatial size baked into the graph — must match exactly.
        if h > 0 && w > 0 {
            return (h as u32).max(w as u32);
        }
    }
    // Dynamic H/W — use a bigger tile so the discrete GPU stays fed.
    configured_tile()
}

impl UpscaleSession {
    fn open(model_path: &Path) -> Result<Self, String> {
        let model_path = model_path
            .canonicalize()
            .map_err(|e| format!("[ERROR: model path] {e}"))?;

        ort::init().with_name("loku").commit();

        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(2, 8);
        let mut builder = Session::builder()
            .map_err(|e| format!("[ERROR: session builder] {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("[ERROR: opt level] {e}"))?
            .with_intra_threads(threads)
            .map_err(|e| format!("[ERROR: intra threads] {e}"))?;

        builder = register_execution_providers(builder, &model_path)?;

        let session = builder
            .commit_from_file(&model_path)
            .map_err(|e| format!("[ERROR: load model] {e}"))?;

        let tile_size = input_tile_size(&session);

        let input_name = session
            .inputs()
            .first()
            .ok_or("[ERROR: model has no inputs]")?
            .name()
            .to_string();
        let output_name = session
            .outputs()
            .first()
            .ok_or("[ERROR: model has no outputs]")?
            .name()
            .to_string();

        Ok(Self {
            session,
            input_name,
            output_name,
            scale: 4,
            tile_size,
        })
    }

    fn detect_scale(&mut self) -> Result<(), String> {
        let img = RgbImage::from_pixel(PROBE_SIZE.min(self.tile_size), PROBE_SIZE.min(self.tile_size), Rgb([128, 128, 128]));
        let input = tile_to_tensor(&pad_tile_to_size(&img, self.tile_size))?;
        let output = self
            .run_tensor(input)
            .map_err(|e| format!("[ERROR: probe inference] {e}"))?;
        let shape = output.shape();
        if shape.len() == 4 && shape[2] > 0 && shape[3] > 0 {
            let sy = shape[2] as u32 / self.tile_size;
            let sx = shape[3] as u32 / self.tile_size;
            self.scale = sy.max(sx).max(1);
        }
        Ok(())
    }

    fn run_tensor(&mut self, input: Array4<f32>) -> Result<Array4<f32>, String> {
        let shape = input.shape().to_vec();
        let data = input.into_raw_vec_and_offset().0;
        let tensor = Tensor::from_array((shape, data))
            .map_err(|e| format!("[ERROR: input tensor] {e}"))?;
        let outputs = self
            .session
            .run(ort::inputs![self.input_name.clone() => tensor])
            .map_err(|e| format!("[ERROR: inference] {e}"))?;
        let out_value = if outputs.contains_key(self.output_name.as_str()) {
            &outputs[self.output_name.as_str()]
        } else {
            &outputs[0]
        };
        let view = out_value
            .try_extract_array::<f32>()
            .map_err(|e| format!("[ERROR: output tensor] {e}"))?;
        let owned = view.to_owned();
        let dims = owned.shape().to_vec();
        let flat = owned.into_raw_vec_and_offset().0;
        Array4::from_shape_vec(
            (dims[0], dims[1], dims[2], dims[3]),
            flat,
        )
        .map_err(|e| format!("[ERROR: output shape] {e}"))
    }

    fn upscale_rgb(
        &mut self,
        img: &RgbImage,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(f32),
    ) -> Result<RgbImage, String> {
        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return Err("[ERROR: empty image]".into());
        }

        let _ = self.detect_scale();

        let scale = self.scale;
        let out_w = w.saturating_mul(scale);
        let out_h = h.saturating_mul(scale);
        let tile = self.tile_size;
        let overlap = configured_overlap(tile);
        let stride = tile.saturating_sub(overlap).max(1);
        let xs = tile_starts(w, tile, stride);
        let ys = tile_starts(h, tile, stride);
        let total = (xs.len() * ys.len()).max(1) as f32;
        let mut done = 0.0f32;

        // Weighted accumulation hides seams in the overlap bands.
        let mut accum = vec![0.0f32; (out_w as usize) * (out_h as usize) * 3];
        let mut weight = vec![0.0f32; (out_w as usize) * (out_h as usize)];
        let overlap_out = overlap.saturating_mul(scale);

        for &y0 in &ys {
            for &x0 in &xs {
                if cancel.load(Ordering::SeqCst) {
                    return Err("[ERROR: cancelled]".into());
                }
                let tile_rgb = extract_tile_reflect(img, x0, y0, tile);
                let input = tile_to_tensor(&tile_rgb)?;
                let output = self.run_tensor(input)?;
                let up = tensor_to_rgb(&output);
                let ox = x0.saturating_mul(scale);
                let oy = y0.saturating_mul(scale);
                blend_tile_into(
                    &mut accum,
                    &mut weight,
                    out_w,
                    out_h,
                    &up,
                    ox,
                    oy,
                    overlap_out,
                );

                done += 1.0;
                progress((done / total) * 100.0);
            }
        }

        Ok(normalize_accum(&accum, &weight, out_w, out_h))
    }
}

fn uses_external_weights(model_path: &Path) -> bool {
    let Some(dir) = model_path.parent() else {
        return false;
    };
    let Some(stem) = model_path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    dir.join(format!("{stem}.data")).is_file()
}

#[cfg(target_os = "macos")]
fn register_execution_providers(
    builder: ort::session::builder::SessionBuilder,
    model_path: &Path,
) -> Result<ort::session::builder::SessionBuilder, String> {
    if uses_external_weights(model_path) {
        return Ok(builder);
    }
    use ort::execution_providers::CoreMLExecutionProvider;
    builder
        .with_execution_providers([CoreMLExecutionProvider::default().build()])
        .map_err(|e| format!("[ERROR: CoreML EP] {e}"))
}

#[cfg(windows)]
fn register_execution_providers(
    builder: ort::session::builder::SessionBuilder,
    model_path: &Path,
) -> Result<ort::session::builder::SessionBuilder, String> {
    if uses_external_weights(model_path) {
        return Ok(builder);
    }
    use ort::ep::directml::{DeviceFilter, PerformancePreference};
    use ort::ep::DirectML;

    // Prefer Windows' high-performance GPU (usually the discrete card). Explicit
    // UPSCALE_GPU still forces a DXGI adapter index for troubleshooting.
    let ep = if let Ok(raw) = std::env::var("UPSCALE_GPU") {
        let device_id: i32 = raw.parse().unwrap_or(0);
        DirectML::default().with_device_id(device_id).build()
    } else {
        DirectML::default()
            .with_device_filter(DeviceFilter::Gpu)
            .with_performance_preference(PerformancePreference::HighPerformance)
            .build()
    };
    builder
        .with_execution_providers([ep])
        .map_err(|e| format!("[ERROR: DirectML EP] {e}"))
}

#[cfg(not(any(target_os = "macos", windows)))]
fn register_execution_providers(
    builder: ort::session::builder::SessionBuilder,
    _model_path: &Path,
) -> Result<ort::session::builder::SessionBuilder, String> {
    Ok(builder)
}

fn reflect_coord(i: i32, len: u32) -> u32 {
    if len == 0 {
        return 0;
    }
    let n = len as i32;
    let mut x = i;
    // Mirror reflect so edge context doesn't clamp-streak.
    while x < 0 || x >= n {
        if x < 0 {
            x = -x - 1;
        } else {
            x = 2 * n - x - 1;
        }
    }
    x as u32
}

fn extract_tile_reflect(img: &RgbImage, x0: u32, y0: u32, size: u32) -> RgbImage {
    let (iw, ih) = img.dimensions();
    let mut tile = RgbImage::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let sx = reflect_coord(x0 as i32 + x as i32, iw);
            let sy = reflect_coord(y0 as i32 + y as i32, ih);
            tile.put_pixel(x, y, *img.get_pixel(sx, sy));
        }
    }
    tile
}

fn pad_tile_to_size(tile: &RgbImage, size: u32) -> RgbImage {
    let (w, h) = tile.dimensions();
    if w >= size && h >= size {
        return tile.clone();
    }
    let mut out = RgbImage::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let sx = x.min(w.saturating_sub(1));
            let sy = y.min(h.saturating_sub(1));
            out.put_pixel(x, y, *tile.get_pixel(sx, sy));
        }
    }
    out
}

fn blend_tile_into(
    accum: &mut [f32],
    weight: &mut [f32],
    out_w: u32,
    out_h: u32,
    tile: &RgbImage,
    ox: u32,
    oy: u32,
    overlap: u32,
) {
    let tw = tile.width().min(out_w.saturating_sub(ox));
    let th = tile.height().min(out_h.saturating_sub(oy));
    for y in 0..th {
        for x in 0..tw {
            let wgt = blend_weight(x, y, tw, th, overlap);
            if wgt <= 0.0 {
                continue;
            }
            let p = tile.get_pixel(x, y).0;
            let idx = ((oy + y) * out_w + (ox + x)) as usize;
            let base = idx * 3;
            accum[base] += p[0] as f32 * wgt;
            accum[base + 1] += p[1] as f32 * wgt;
            accum[base + 2] += p[2] as f32 * wgt;
            weight[idx] += wgt;
        }
    }
}

fn normalize_accum(accum: &[f32], weight: &[f32], out_w: u32, out_h: u32) -> RgbImage {
    let mut out = RgbImage::new(out_w, out_h);
    for y in 0..out_h {
        for x in 0..out_w {
            let idx = (y * out_w + x) as usize;
            let w = weight[idx];
            let (r, g, b) = if w > 1e-6 {
                let base = idx * 3;
                (
                    (accum[base] / w).round().clamp(0.0, 255.0) as u8,
                    (accum[base + 1] / w).round().clamp(0.0, 255.0) as u8,
                    (accum[base + 2] / w).round().clamp(0.0, 255.0) as u8,
                )
            } else {
                (0, 0, 0)
            };
            out.put_pixel(x, y, Rgb([r, g, b]));
        }
    }
    out
}

fn tile_to_tensor(img: &RgbImage) -> Result<Array4<f32>, String> {
    let (w, h) = img.dimensions();
    let mut data = Vec::with_capacity((w * h * 3) as usize);
    for c in 0..3 {
        for y in 0..h {
            for x in 0..w {
                let v = img.get_pixel(x, y)[c as usize] as f32 / 255.0;
                data.push(v);
            }
        }
    }
    Array4::from_shape_vec((1, 3, h as usize, w as usize), data)
        .map_err(|e| format!("[ERROR: tensor shape] {e}"))
}

fn tensor_to_rgb(output: &Array4<f32>) -> RgbImage {
    let h = output.shape()[2];
    let w = output.shape()[3];
    let mut img = RgbImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let r = (output[[0, 0, y, x]].clamp(0.0, 1.0) * 255.0).round() as u8;
            let g = (output[[0, 1, y, x]].clamp(0.0, 1.0) * 255.0).round() as u8;
            let b = (output[[0, 2, y, x]].clamp(0.0, 1.0) * 255.0).round() as u8;
            img.put_pixel(x as u32, y as u32, Rgb([r, g, b]));
        }
    }
    img
}

pub fn upscale_file(
    model_path: &Path,
    input: &Path,
    output: &Path,
    format: OutputFormat,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f32),
) -> Result<(), String> {
    let img = image::open(input).map_err(|e| format!("[ERROR: read input] {e}"))?;
    let rgb = img.to_rgb8();
    let mut session = UpscaleSession::open(model_path)?;
    let out = session.upscale_rgb(&rgb, cancel, progress)?;
    save_image(&DynamicImage::ImageRgb8(out), output, format)
}

fn save_image(img: &DynamicImage, path: &Path, format: OutputFormat) -> Result<(), String> {
    match format {
        OutputFormat::Png => img
            .save_with_format(path, image::ImageFormat::Png)
            .map_err(|e| e.to_string()),
        OutputFormat::Jpg => img
            .save_with_format(path, image::ImageFormat::Jpeg)
            .map_err(|e| e.to_string()),
        OutputFormat::Webp => img
            .save_with_format(path, image::ImageFormat::WebP)
            .map_err(|e| e.to_string()),
    }
    .map_err(|e| format!("[ERROR: write output] {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn tile_starts_covers_end() {
        assert_eq!(tile_starts(100, 256, 224), vec![0]);
        assert_eq!(tile_starts(500, 256, 224), vec![0, 224, 244]);
        assert_eq!(tile_starts(512, 256, 224), vec![0, 224, 256]);
    }

    #[test]
    fn blend_weight_ramps_at_edges() {
        assert!((blend_weight(0, 50, 100, 100, 10) - 0.1).abs() < 1e-5);
        assert!((blend_weight(50, 50, 100, 100, 10) - 1.0).abs() < 1e-5);
        assert!(blend_weight(5, 5, 100, 100, 10) < 1.0);
    }

    #[test]
    fn reflect_coord_mirrors() {
        assert_eq!(reflect_coord(-1, 10), 0);
        assert_eq!(reflect_coord(10, 10), 9);
        assert_eq!(reflect_coord(5, 10), 5);
    }

    #[test]
    fn smoke_upscale_if_model_present() {
        let onnx_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tools/upscale/models/onnx");
        let model = ["4x-UltraSharpV2.onnx", "4xPurePhoto-RealPLSKR.onnx"]
            .iter()
            .map(|n| onnx_dir.join(n))
            .find(|p| p.is_file())
            .unwrap_or_else(|| onnx_dir.join("missing.onnx"));
        if !model.is_file() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("in.png");
        let output = tmp.path().join("out.png");
        DynamicImage::ImageRgb8(RgbImage::from_pixel(64, 64, Rgb([100, 120, 140])))
            .save(&input)
            .unwrap();
        let cancel = AtomicBool::new(false);
        upscale_file(&model, &input, &output, OutputFormat::Png, &cancel, &mut |_| {})
            .expect("onnx upscale");
        assert!(output.is_file());
    }
}
