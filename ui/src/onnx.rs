use crate::models::OutputFormat;
use image::{DynamicImage, Rgb, RgbImage};
use ndarray::Array4;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::{Tensor, ValueType};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const DEFAULT_TILE: u32 = 128;
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
    if name.contains("anime") {
        "ONNX — anime / illustration model"
    } else if name.contains("hat") {
        "ONNX — HAT transformer, high-quality photos (FP32)"
    } else if name.contains("dat") || name.contains("nomos") {
        "ONNX — DAT / Nomos8k, detailed photo upscale"
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

fn input_tile_size(session: &Session) -> u32 {
    let Some(input) = session.inputs().first() else {
        return DEFAULT_TILE;
    };
    let ValueType::Tensor { shape, .. } = input.dtype() else {
        return DEFAULT_TILE;
    };
    let dims: Vec<i64> = shape.iter().copied().collect();
    if dims.len() == 4 {
        let h = dims[2];
        let w = dims[3];
        if h > 0 && w > 0 {
            return (h as u32).max(w as u32);
        }
    }
    DEFAULT_TILE
}

impl UpscaleSession {
    fn open(model_path: &Path) -> Result<Self, String> {
        let model_path = model_path
            .canonicalize()
            .map_err(|e| format!("[ERROR: model path] {e}"))?;

        ort::init().with_name("loku").commit();

        let mut builder = Session::builder()
            .map_err(|e| format!("[ERROR: session builder] {e}"))?
            .with_optimization_level(GraphOptimizationLevel::Disable)
            .map_err(|e| format!("[ERROR: opt level] {e}"))?;

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
        let mut out = RgbImage::new(out_w, out_h);

        let tile = self.tile_size;
        let tiles_x = (w + tile - 1) / tile;
        let tiles_y = (h + tile - 1) / tile;
        let total = (tiles_x * tiles_y).max(1) as f32;
        let mut done = 0.0f32;

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                if cancel.load(Ordering::SeqCst) {
                    return Err("[ERROR: cancelled]".into());
                }
                let x0 = tx * tile;
                let y0 = ty * tile;
                let tw = tile.min(w - x0);
                let th = tile.min(h - y0);
                let tile_rgb = pad_tile_to_size(&extract_tile_rgb(img, x0, y0, tw, th), tile);
                let input = tile_to_tensor(&tile_rgb)?;
                let output = self.run_tensor(input)?;

                paste_tile(
                    &mut out,
                    &tensor_to_rgb(&output),
                    x0.saturating_mul(scale),
                    y0.saturating_mul(scale),
                    tw.saturating_mul(scale),
                    th.saturating_mul(scale),
                );

                done += 1.0;
                progress((done / total) * 100.0);
            }
        }

        Ok(out)
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
    use ort::execution_providers::DirectMLExecutionProvider;
    builder
        .with_execution_providers([DirectMLExecutionProvider::default().with_device_id(0).build()])
        .map_err(|e| format!("[ERROR: DirectML EP] {e}"))
}

#[cfg(not(any(target_os = "macos", windows)))]
fn register_execution_providers(
    builder: ort::session::builder::SessionBuilder,
    _model_path: &Path,
) -> Result<ort::session::builder::SessionBuilder, String> {
    Ok(builder)
}

fn extract_tile_rgb(img: &RgbImage, x0: u32, y0: u32, w: u32, h: u32) -> RgbImage {
    let mut tile = RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            tile.put_pixel(x, y, *img.get_pixel(x0 + x, y0 + y));
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

fn paste_tile(out: &mut RgbImage, tile: &RgbImage, ox: u32, oy: u32, core_w: u32, core_h: u32) {
    for y in 0..core_h.min(tile.height()).min(out.height().saturating_sub(oy)) {
        for x in 0..core_w.min(tile.width()).min(out.width().saturating_sub(ox)) {
            out.put_pixel(ox + x, oy + y, *tile.get_pixel(x, y));
        }
    }
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
    fn smoke_upscale_if_model_present() {
        let model = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/upscale/models/onnx/real_esrgan_x4plus.onnx");
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
