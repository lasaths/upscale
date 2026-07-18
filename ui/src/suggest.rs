//! Suggest upscaler settings from a lightweight medium classifier (anime / real / rendered).
//!
//! Model: ONNX export of Mitchins/image-medium-classifier-efficientnet-b0-v1 (OpenRAIL).

use crate::models::{
    Algorithm, DenoiseLevel, RealEsrganModel, Variant, Waifu2xModel,
};
use crate::paths::BackendPaths;
use image::imageops::FilterType;
use image::GenericImageView;
use ndarray::Array4;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor;
use std::path::{Path, PathBuf};

const INPUT_SIZE: u32 = 224;
const CONFIDENCE_SOFT: f32 = 0.85;
const IMAGENET_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const IMAGENET_STD: [f32; 3] = [0.229, 0.224, 0.225];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContentClass {
    Anime,
    Real,
    Rendered,
}

impl ContentClass {
    pub fn label(self) -> &'static str {
        match self {
            ContentClass::Anime => "anime",
            ContentClass::Real => "real",
            ContentClass::Rendered => "rendered",
        }
    }

    fn from_index(i: usize) -> Option<Self> {
        match i {
            0 => Some(ContentClass::Anime),
            1 => Some(ContentClass::Real),
            2 => Some(ContentClass::Rendered),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClassifyResult {
    pub class: ContentClass,
    pub confidence: f32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct SuggestedSettings {
    pub algorithm: Algorithm,
    pub variant: Variant,
    pub scale: u8,
    pub denoise: DenoiseLevel,
    pub class: ContentClass,
    pub confidence: f32,
    pub low_confidence: bool,
}

impl SuggestedSettings {
    pub fn status_line(&self) -> String {
        let engine = self.algorithm.label();
        let model = variant_short_label(self.variant);
        let mut line = if self.low_confidence {
            format!(
                "Low confidence ({:.0}%) · Detected {} · {} {} · {}×",
                self.confidence * 100.0,
                self.class.label(),
                engine,
                model,
                self.scale
            )
        } else {
            format!(
                "Detected {} · {} {} · {}×",
                self.class.label(),
                engine,
                model,
                self.scale
            )
        };
        if self.algorithm.supports_denoise() {
            line.push_str(&format!(" · denoise {}", self.denoise.label()));
        }
        line
    }
}

fn variant_short_label(variant: Variant) -> &'static str {
    match variant {
        Variant::RealEsrgan(RealEsrganModel::AnimeV3) => "animev3",
        Variant::RealEsrgan(RealEsrganModel::X4Plus) => "x4plus",
        Variant::RealEsrgan(RealEsrganModel::X4PlusAnime) => "x4-anime",
        Variant::RealEsrgan(RealEsrganModel::X4Net) => "x4net",
        Variant::RealCugan(_) => "se",
        Variant::Waifu2x(Waifu2xModel::Cunet) => "cunet",
        Variant::Waifu2x(Waifu2xModel::UpconvAnime) => "anime",
        Variant::Waifu2x(Waifu2xModel::UpconvPhoto) => "photo",
        Variant::RealSr(_) => "df2k",
    }
}

pub fn discover_model(models_root: &Path) -> Option<PathBuf> {
    let path = models_root.join("suggest/medium_classify.onnx");
    path.is_file().then_some(path)
}

/// Classify an image file. Opens a short-lived CPU ORT session.
pub fn classify(model_path: &Path, image_path: &Path) -> Result<ClassifyResult, String> {
    let img = image::open(image_path).map_err(|e| format!("[ERROR: read image] {e}"))?;
    let (width, height) = img.dimensions();
    let rgb = img
        .resize_exact(INPUT_SIZE, INPUT_SIZE, FilterType::Triangle)
        .to_rgb8();

    let mut tensor = Array4::<f32>::zeros((1, 3, INPUT_SIZE as usize, INPUT_SIZE as usize));
    for y in 0..INPUT_SIZE {
        for x in 0..INPUT_SIZE {
            let p = rgb.get_pixel(x, y).0;
            for c in 0..3 {
                let v = p[c] as f32 / 255.0;
                tensor[[0, c, y as usize, x as usize]] =
                    (v - IMAGENET_MEAN[c]) / IMAGENET_STD[c];
            }
        }
    }

    ort::init().with_name("loku-suggest").commit();

    let mut session = Session::builder()
        .map_err(|e| format!("[ERROR: session builder] {e}"))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| format!("[ERROR: opt level] {e}"))?
        .commit_from_file(model_path)
        .map_err(|e| format!("[ERROR: load suggest model] {e}"))?;

    let input_name = session
        .inputs()
        .first()
        .ok_or("[ERROR: suggest model has no inputs]")?
        .name()
        .to_string();

    let shape = tensor.shape().to_vec();
    let data = tensor.into_raw_vec_and_offset().0;
    let input = Tensor::from_array((shape, data))
        .map_err(|e| format!("[ERROR: input tensor] {e}"))?;

    let outputs = session
        .run(ort::inputs![input_name => input])
        .map_err(|e| format!("[ERROR: suggest inference] {e}"))?;

    let out_value = &outputs[0];
    let view = out_value
        .try_extract_array::<f32>()
        .map_err(|e| format!("[ERROR: output tensor] {e}"))?;
    let logits: Vec<f32> = view.iter().copied().collect();
    if logits.len() < 3 {
        return Err(format!(
            "[ERROR: unexpected logits len {}]",
            logits.len()
        ));
    }

    let (idx, confidence) = softmax_argmax(&logits[..3]);
    let class = ContentClass::from_index(idx)
        .ok_or_else(|| format!("[ERROR: bad class index {idx}]"))?;

    Ok(ClassifyResult {
        class,
        confidence,
        width,
        height,
    })
}

fn softmax_argmax(logits: &[f32]) -> (usize, f32) {
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = logits.iter().map(|v| (v - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    let mut best_i = 0;
    let mut best_p = 0.0f32;
    for (i, e) in exps.iter().enumerate() {
        let p = e / sum;
        if p > best_p {
            best_p = p;
            best_i = i;
        }
    }
    (best_i, best_p)
}

pub fn suggest_settings(
    result: &ClassifyResult,
    backends: &BackendPaths,
) -> Result<SuggestedSettings, String> {
    let (algorithm, variant, denoise) = pick_preset(result.class, backends)?;
    let scale = suggest_scale(result.class, result.width, result.height, algorithm);
    Ok(SuggestedSettings {
        algorithm,
        variant,
        scale,
        denoise,
        class: result.class,
        confidence: result.confidence,
        low_confidence: result.confidence < CONFIDENCE_SOFT,
    })
}

fn pick_preset(
    class: ContentClass,
    backends: &BackendPaths,
) -> Result<(Algorithm, Variant, DenoiseLevel), String> {
    match class {
        ContentClass::Anime => {
            if backends.realesrgan.is_some() {
                Ok((
                    Algorithm::RealEsrgan,
                    Variant::RealEsrgan(RealEsrganModel::AnimeV3),
                    DenoiseLevel::Zero,
                ))
            } else if backends.realcugan.is_some() {
                Ok((
                    Algorithm::RealCugan,
                    Algorithm::RealCugan.default_variant(),
                    DenoiseLevel::Minus1,
                ))
            } else if backends.waifu2x.is_some() {
                Ok((
                    Algorithm::Waifu2x,
                    Variant::Waifu2x(Waifu2xModel::Cunet),
                    DenoiseLevel::Zero,
                ))
            } else {
                Err("[ERROR: no anime-capable engine installed]".into())
            }
        }
        ContentClass::Real | ContentClass::Rendered => {
            if backends.realesrgan.is_some() {
                Ok((
                    Algorithm::RealEsrgan,
                    Variant::RealEsrgan(RealEsrganModel::X4Plus),
                    DenoiseLevel::Zero,
                ))
            } else if backends.realsr.is_some() {
                Ok((
                    Algorithm::RealSr,
                    Algorithm::RealSr.default_variant(),
                    DenoiseLevel::Zero,
                ))
            } else if backends.waifu2x.is_some() {
                Ok((
                    Algorithm::Waifu2x,
                    Variant::Waifu2x(Waifu2xModel::UpconvPhoto),
                    DenoiseLevel::Zero,
                ))
            } else {
                Err("[ERROR: no photo-capable engine installed]".into())
            }
        }
    }
}

pub fn suggest_scale(class: ContentClass, width: u32, height: u32, algorithm: Algorithm) -> u8 {
    let max_edge = width.max(height);
    let preferred = if max_edge < 512 {
        4
    } else if max_edge < 1024 {
        3
    } else {
        match class {
            ContentClass::Anime => 2,
            ContentClass::Real | ContentClass::Rendered => 4,
        }
    };
    algorithm.clamp_scale(preferred)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Backend;
    use std::path::PathBuf;

    fn backends_with(algos: &[Algorithm]) -> BackendPaths {
        let stub = || Backend {
            exe: PathBuf::from("stub"),
            models_root: PathBuf::from("models"),
        };
        let mut b = BackendPaths::default();
        for a in algos {
            match a {
                Algorithm::RealEsrgan => b.realesrgan = Some(stub()),
                Algorithm::RealCugan => b.realcugan = Some(stub()),
                Algorithm::Waifu2x => b.waifu2x = Some(stub()),
                Algorithm::RealSr => b.realsr = Some(stub()),
                Algorithm::Onnx => {}
            }
        }
        b
    }

    #[test]
    fn anime_prefers_esrgan_animev3() {
        let b = backends_with(&[Algorithm::RealEsrgan, Algorithm::Waifu2x]);
        let r = ClassifyResult {
            class: ContentClass::Anime,
            confidence: 0.95,
            width: 400,
            height: 400,
        };
        let s = suggest_settings(&r, &b).unwrap();
        assert_eq!(s.algorithm, Algorithm::RealEsrgan);
        assert_eq!(s.variant, Variant::RealEsrgan(RealEsrganModel::AnimeV3));
        assert_eq!(s.scale, 4);
        assert!(!s.low_confidence);
    }

    #[test]
    fn real_falls_back_to_realsr() {
        let b = backends_with(&[Algorithm::RealSr]);
        let r = ClassifyResult {
            class: ContentClass::Real,
            confidence: 0.99,
            width: 800,
            height: 600,
        };
        let s = suggest_settings(&r, &b).unwrap();
        assert_eq!(s.algorithm, Algorithm::RealSr);
        assert_eq!(s.scale, 4);
    }

    #[test]
    fn low_confidence_flag() {
        let b = backends_with(&[Algorithm::RealEsrgan]);
        let r = ClassifyResult {
            class: ContentClass::Rendered,
            confidence: 0.7,
            width: 1920,
            height: 1080,
        };
        let s = suggest_settings(&r, &b).unwrap();
        assert!(s.low_confidence);
        assert!(s.status_line().starts_with("Low confidence"));
    }

    #[test]
    fn scale_heuristic_large_anime_is_2x() {
        assert_eq!(
            suggest_scale(ContentClass::Anime, 1600, 900, Algorithm::RealEsrgan),
            2
        );
    }

    #[test]
    fn scale_heuristic_mid_clamps_for_waifu2x() {
        // preferred 3 → waifu2x clamps to 4
        assert_eq!(
            suggest_scale(ContentClass::Anime, 800, 800, Algorithm::Waifu2x),
            4
        );
    }

    #[test]
    fn anime_fallback_cugan_denoise() {
        let b = backends_with(&[Algorithm::RealCugan]);
        let r = ClassifyResult {
            class: ContentClass::Anime,
            confidence: 0.9,
            width: 300,
            height: 300,
        };
        let s = suggest_settings(&r, &b).unwrap();
        assert_eq!(s.algorithm, Algorithm::RealCugan);
        assert_eq!(s.denoise, DenoiseLevel::Minus1);
        assert!(s.status_line().contains("denoise"));
    }

    #[test]
    fn softmax_picks_peak() {
        let (i, p) = softmax_argmax(&[1.0, 5.0, 1.0]);
        assert_eq!(i, 1);
        assert!(p > 0.9);
    }

    #[test]
    fn smoke_classify_if_model_present() {
        let model = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tools/upscale/models/suggest/medium_classify.onnx");
        if !model.is_file() {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("in.png");
        image::RgbImage::from_pixel(96, 96, image::Rgb([210, 90, 140]))
            .save(&input)
            .unwrap();
        let result = classify(&model, &input).expect("classify");
        assert!(result.confidence.is_finite());
        assert!(result.confidence > 0.0);
    }
}
