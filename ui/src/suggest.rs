//! Suggest upscaler settings via a deepghs cascade:
//! anime_real → (if anime) anime_classification → anime vs photo preset.
//!
//! Models (OpenRAIL ONNX, downloaded by setup):
//! - deepghs/anime_real_cls · mobilenetv3_v1.4_dist
//! - deepghs/anime_classification · mobilenetv3_v1.5_dist

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

const INPUT_SIZE: u32 = 384;
const CONFIDENCE_SOFT: f32 = 0.85;
/// deepghs `_img_encode` normalize `(0.5, 0.5)` → `(x - 0.5) / 0.5`.
const NORM_MEAN: f32 = 0.5;
const NORM_STD: f32 = 0.5;

const ANIME_REAL_NAME: &str = "anime_real.onnx";
const ANIME_CLS_NAME: &str = "anime_cls.onnx";

#[derive(Clone, Debug)]
pub struct SuggestModels {
    pub anime_real: PathBuf,
    pub anime_cls: PathBuf,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ContentClass {
    Anime,
    Photo,
}

impl ContentClass {
    pub fn label(self) -> &'static str {
        match self {
            ContentClass::Anime => "anime",
            ContentClass::Photo => "photo",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClassifyResult {
    pub class: ContentClass,
    /// Subtype for status: illustration / bangumi / comic / real / 3d / not_painting.
    pub detail: &'static str,
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
    pub detail: &'static str,
    pub confidence: f32,
    pub low_confidence: bool,
}

impl SuggestedSettings {
    pub fn status_line(&self) -> String {
        let engine = self.algorithm.label();
        let model = variant_short_label(self.variant);
        let detected = if self.detail == self.class.label() {
            self.class.label().to_string()
        } else {
            format!("{} ({})", self.class.label(), self.detail)
        };
        let mut line = if self.low_confidence {
            format!(
                "Low confidence ({:.0}%) · Detected {} · {} {} · {}×",
                self.confidence * 100.0,
                detected,
                engine,
                model,
                self.scale
            )
        } else {
            format!(
                "Detected {} · {} {} · {}×",
                detected, engine, model, self.scale
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

pub fn discover_models(models_root: &Path) -> Option<SuggestModels> {
    let dir = models_root.join("suggest");
    let anime_real = dir.join(ANIME_REAL_NAME);
    let anime_cls = dir.join(ANIME_CLS_NAME);
    if anime_real.is_file() && anime_cls.is_file() {
        Some(SuggestModels {
            anime_real,
            anime_cls,
        })
    } else {
        None
    }
}

/// Cascade-classify an image. Opens short-lived CPU ORT sessions.
pub fn classify(models: &SuggestModels, image_path: &Path) -> Result<ClassifyResult, String> {
    let img = image::open(image_path).map_err(|e| format!("[ERROR: read image] {e}"))?;
    let (width, height) = img.dimensions();
    let tensor = encode_deepghs(&img)?;

    ort::init().with_name("loku-suggest").commit();

    let real_logits = run_onnx(&models.anime_real, &tensor)?;
    if real_logits.len() < 2 {
        return Err(format!(
            "[ERROR: anime_real logits len {}]",
            real_logits.len()
        ));
    }
    // labels: anime, real
    let (real_idx, real_conf) = peak_prob(&real_logits[..2]);

    if real_idx == 1 {
        return Ok(ClassifyResult {
            class: ContentClass::Photo,
            detail: "real",
            confidence: real_conf,
            width,
            height,
        });
    }

    let cls_logits = run_onnx(&models.anime_cls, &tensor)?;
    if cls_logits.len() < 5 {
        return Err(format!(
            "[ERROR: anime_cls logits len {}]",
            cls_logits.len()
        ));
    }
    // labels: 3d, bangumi, comic, illustration, not_painting
    let (cls_idx, cls_conf) = peak_prob(&cls_logits[..5]);
    let detail = match cls_idx {
        0 => "3d",
        1 => "bangumi",
        2 => "comic",
        3 => "illustration",
        4 => "not_painting",
        _ => "anime",
    };
    let confidence = real_conf.min(cls_conf);

    // Photo bucket: CG / UI / non-illustration → x4plus. Anime art → animev3.
    let class = match detail {
        "illustration" | "bangumi" | "comic" => ContentClass::Anime,
        _ => ContentClass::Photo, // 3d, not_painting
    };

    Ok(ClassifyResult {
        class,
        detail,
        confidence,
        width,
        height,
    })
}

fn encode_deepghs(img: &image::DynamicImage) -> Result<Array4<f32>, String> {
    let rgb = img
        .resize_exact(INPUT_SIZE, INPUT_SIZE, FilterType::Triangle)
        .to_rgb8();
    let mut tensor = Array4::<f32>::zeros((1, 3, INPUT_SIZE as usize, INPUT_SIZE as usize));
    for y in 0..INPUT_SIZE {
        for x in 0..INPUT_SIZE {
            let p = rgb.get_pixel(x, y).0;
            for c in 0..3 {
                let v = p[c] as f32 / 255.0;
                tensor[[0, c, y as usize, x as usize]] = (v - NORM_MEAN) / NORM_STD;
            }
        }
    }
    Ok(tensor)
}

fn run_onnx(model_path: &Path, tensor: &Array4<f32>) -> Result<Vec<f32>, String> {
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
    let data = tensor.as_slice().ok_or("[ERROR: tensor not contiguous]")?.to_vec();
    let input = Tensor::from_array((shape, data))
        .map_err(|e| format!("[ERROR: input tensor] {e}"))?;

    let outputs = session
        .run(ort::inputs![input_name => input])
        .map_err(|e| format!("[ERROR: suggest inference] {e}"))?;

    let view = outputs[0]
        .try_extract_array::<f32>()
        .map_err(|e| format!("[ERROR: output tensor] {e}"))?;
    Ok(view.iter().copied().collect())
}

/// Prefer raw probs (deepghs models emit them); fall back to softmax for logits.
fn peak_prob(scores: &[f32]) -> (usize, f32) {
    let all_nonneg = scores.iter().all(|&v| v >= 0.0);
    let sum: f32 = scores.iter().sum();
    if all_nonneg && (sum - 1.0).abs() < 0.05 {
        let mut best_i = 0;
        let mut best_p = 0.0f32;
        for (i, &p) in scores.iter().enumerate() {
            if p > best_p {
                best_p = p;
                best_i = i;
            }
        }
        return (best_i, best_p);
    }
    softmax_argmax(scores)
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
        detail: result.detail,
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
        ContentClass::Photo => {
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
            ContentClass::Photo => 4,
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
            detail: "illustration",
            confidence: 0.95,
            width: 400,
            height: 400,
        };
        let s = suggest_settings(&r, &b).unwrap();
        assert_eq!(s.algorithm, Algorithm::RealEsrgan);
        assert_eq!(s.variant, Variant::RealEsrgan(RealEsrganModel::AnimeV3));
        assert_eq!(s.scale, 4);
        assert!(!s.low_confidence);
        assert!(s.status_line().contains("illustration"));
    }

    #[test]
    fn photo_falls_back_to_realsr() {
        let b = backends_with(&[Algorithm::RealSr]);
        let r = ClassifyResult {
            class: ContentClass::Photo,
            detail: "real",
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
            class: ContentClass::Photo,
            detail: "3d",
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
            detail: "comic",
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
    fn peak_prob_uses_raw_when_normalized() {
        let (i, p) = peak_prob(&[0.1, 0.9]);
        assert_eq!(i, 1);
        assert!((p - 0.9).abs() < 1e-5);
    }

    #[test]
    fn discover_requires_both_models() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("models");
        let dir = root.join("suggest");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(discover_models(&root).is_none());
        std::fs::write(dir.join(ANIME_REAL_NAME), b"x").unwrap();
        assert!(discover_models(&root).is_none());
        std::fs::write(dir.join(ANIME_CLS_NAME), b"x").unwrap();
        let m = discover_models(&root).unwrap();
        assert!(m.anime_real.ends_with(ANIME_REAL_NAME));
        assert!(m.anime_cls.ends_with(ANIME_CLS_NAME));
    }

    #[test]
    fn smoke_classify_if_models_present() {
        let models_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tools/upscale/models");
        let Some(models) = discover_models(&models_root) else {
            return;
        };
        let tmp = tempfile::tempdir().unwrap();
        let input = tmp.path().join("in.png");
        image::RgbImage::from_pixel(96, 96, image::Rgb([210, 90, 140]))
            .save(&input)
            .unwrap();
        let result = classify(&models, &input).expect("classify");
        assert!(result.confidence.is_finite());
        assert!(result.confidence > 0.0);
    }
}
