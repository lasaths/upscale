use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Algorithm {
    #[default]
    RealEsrgan,
    RealCugan,
    Waifu2x,
    RealSr,
    Onnx,
}

impl Algorithm {
    pub const NCNN: [Algorithm; 4] = [
        Algorithm::RealEsrgan,
        Algorithm::RealCugan,
        Algorithm::Waifu2x,
        Algorithm::RealSr,
    ];

    pub const ALL: [Algorithm; 5] = [
        Algorithm::RealEsrgan,
        Algorithm::RealCugan,
        Algorithm::Waifu2x,
        Algorithm::RealSr,
        Algorithm::Onnx,
    ];

    pub fn is_onnx(self) -> bool {
        matches!(self, Algorithm::Onnx)
    }

    pub fn label(self) -> &'static str {
        match self {
            Algorithm::RealEsrgan => "ESRGAN",
            Algorithm::RealCugan => "CUGAN",
            Algorithm::Waifu2x => "WAIFU2",
            Algorithm::RealSr => "REALSR",
            Algorithm::Onnx => "ONNX",
        }
    }

    pub fn header_label(self) -> &'static str {
        match self {
            Algorithm::RealEsrgan => "realesrgan",
            Algorithm::RealCugan => "realcugan",
            Algorithm::Waifu2x => "waifu2x",
            Algorithm::RealSr => "realsr",
            Algorithm::Onnx => "onnx",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Algorithm::RealEsrgan => "General-purpose — photos, web images, and anime",
            Algorithm::RealCugan => "Anime & illustrations — clean lines, denoise control",
            Algorithm::Waifu2x => "Classic anime upscaler — fast, great for line art",
            Algorithm::RealSr => "Real-world photos — natural detail at 4×",
            Algorithm::Onnx => {
                "Higher-quality models via ONNX Runtime (CoreML on Mac, DirectML on Windows)"
            }
        }
    }

    pub fn exe_name(self) -> &'static str {
        if cfg!(windows) {
            match self {
                Algorithm::RealEsrgan => "realesrgan-ncnn-vulkan.exe",
                Algorithm::RealCugan => "realcugan-ncnn-vulkan.exe",
                Algorithm::Waifu2x => "waifu2x-ncnn-vulkan.exe",
                Algorithm::RealSr => "realsr-ncnn-vulkan.exe",
                Algorithm::Onnx => "onnx",
            }
        } else {
            match self {
                Algorithm::RealEsrgan => "realesrgan-ncnn-vulkan",
                Algorithm::RealCugan => "realcugan-ncnn-vulkan",
                Algorithm::Waifu2x => "waifu2x-ncnn-vulkan",
                Algorithm::RealSr => "realsr-ncnn-vulkan",
                Algorithm::Onnx => "onnx",
            }
        }
    }

    pub fn valid_scales(self) -> &'static [u8] {
        match self {
            Algorithm::RealEsrgan | Algorithm::RealCugan => &[2, 3, 4],
            Algorithm::Waifu2x => &[2, 4],
            Algorithm::RealSr | Algorithm::Onnx => &[4],
        }
    }

    pub fn clamp_scale(self, scale: u8) -> u8 {
        let scales = self.valid_scales();
        if scales.contains(&scale) {
            scale
        } else {
            *scales.last().unwrap_or(&4)
        }
    }

    pub fn default_scale(self) -> u8 {
        match self {
            Algorithm::RealEsrgan | Algorithm::RealCugan | Algorithm::RealSr | Algorithm::Onnx => {
                4
            }
            Algorithm::Waifu2x => 2,
        }
    }

    pub fn supports_denoise(self) -> bool {
        matches!(self, Algorithm::RealCugan | Algorithm::Waifu2x)
    }

    pub fn supports_tta(self) -> bool {
        !self.is_onnx()
    }

    pub fn default_denoise(self) -> DenoiseLevel {
        match self {
            Algorithm::RealCugan => DenoiseLevel::Minus1,
            Algorithm::Waifu2x => DenoiseLevel::Zero,
            _ => DenoiseLevel::Zero,
        }
    }

    pub fn default_variant(self) -> Variant {
        match self {
            Algorithm::RealEsrgan => Variant::RealEsrgan(RealEsrganModel::X4Plus),
            Algorithm::RealCugan => Variant::RealCugan(RealCuganModel::Se),
            Algorithm::Waifu2x => Variant::Waifu2x(Waifu2xModel::Cunet),
            Algorithm::RealSr => Variant::RealSr(RealSrModel::Df2k),
            Algorithm::Onnx => Variant::RealEsrgan(RealEsrganModel::X4Plus),
        }
    }

    pub fn variant_options(self) -> &'static [(Variant, &'static str)] {
        match self {
            Algorithm::Onnx => &[],
            Algorithm::RealEsrgan => &[
                (Variant::RealEsrgan(RealEsrganModel::AnimeV3), "animev3"),
                (Variant::RealEsrgan(RealEsrganModel::X4Plus), "x4plus"),
                (
                    Variant::RealEsrgan(RealEsrganModel::X4PlusAnime),
                    "x4-anime",
                ),
                (Variant::RealEsrgan(RealEsrganModel::X4Net), "x4net"),
            ],
            Algorithm::RealCugan => &[(Variant::RealCugan(RealCuganModel::Se), "se")],
            Algorithm::Waifu2x => &[
                (Variant::Waifu2x(Waifu2xModel::Cunet), "cunet"),
                (Variant::Waifu2x(Waifu2xModel::UpconvAnime), "anime"),
                (Variant::Waifu2x(Waifu2xModel::UpconvPhoto), "photo"),
            ],
            Algorithm::RealSr => &[(Variant::RealSr(RealSrModel::Df2k), "df2k")],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RealEsrganModel {
    AnimeV3,
    X4Plus,
    X4PlusAnime,
    X4Net,
}

impl RealEsrganModel {
    pub fn cli_name(self) -> &'static str {
        match self {
            RealEsrganModel::AnimeV3 => "realesr-animevideov3",
            RealEsrganModel::X4Plus => "realesrgan-x4plus",
            RealEsrganModel::X4PlusAnime => "realesrgan-x4plus-anime",
            RealEsrganModel::X4Net => "realesrnet-x4plus",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RealCuganModel {
    Se,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Waifu2xModel {
    Cunet,
    UpconvAnime,
    UpconvPhoto,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RealSrModel {
    Df2k,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    RealEsrgan(RealEsrganModel),
    RealCugan(RealCuganModel),
    Waifu2x(Waifu2xModel),
    RealSr(RealSrModel),
}

impl Variant {
    #[allow(dead_code)]
    pub fn algorithm(self) -> Algorithm {
        match self {
            Variant::RealEsrgan(_) => Algorithm::RealEsrgan,
            Variant::RealCugan(_) => Algorithm::RealCugan,
            Variant::Waifu2x(_) => Algorithm::Waifu2x,
            Variant::RealSr(_) => Algorithm::RealSr,
        }
    }

    /// Model directory passed to `-m` (relative to the shared models root).
    /// Names must match what each ncnn binary expects (folder name detection).
    pub fn model_subdir(self) -> &'static str {
        match self {
            Variant::RealEsrgan(_) => "realesrgan",
            Variant::RealCugan(RealCuganModel::Se) => "models-se",
            Variant::Waifu2x(Waifu2xModel::Cunet) => "models-cunet",
            Variant::Waifu2x(Waifu2xModel::UpconvAnime) => "models-upconv_7_anime_style_art_rgb",
            Variant::Waifu2x(Waifu2xModel::UpconvPhoto) => "models-upconv_7_photo",
            Variant::RealSr(RealSrModel::Df2k) => "models-DF2K",
        }
    }

    pub fn esrgan_cli_name(self) -> Option<&'static str> {
        match self {
            Variant::RealEsrgan(m) => Some(m.cli_name()),
            _ => None,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Variant::RealEsrgan(RealEsrganModel::AnimeV3) => {
                "Anime & video frames — flexible 2×, 3×, 4×"
            }
            Variant::RealEsrgan(RealEsrganModel::X4Plus) => {
                "Best default for natural photos and mixed content"
            }
            Variant::RealEsrgan(RealEsrganModel::X4PlusAnime) => {
                "Tuned for anime stills and flat-color art"
            }
            Variant::RealEsrgan(RealEsrganModel::X4Net) => {
                "Smoother look — less sharpening, fewer GAN artifacts"
            }
            Variant::RealCugan(RealCuganModel::Se) => "Standard Real-CUGAN preset for anime",
            Variant::Waifu2x(Waifu2xModel::Cunet) => "Highest-quality waifu2x — most anime use cases",
            Variant::Waifu2x(Waifu2xModel::UpconvAnime) => {
                "Lighter waifu2x model for anime-style RGB art"
            }
            Variant::Waifu2x(Waifu2xModel::UpconvPhoto) => "Waifu2x variant for photographic input",
            Variant::RealSr(RealSrModel::Df2k) => "DF2K-trained — real-world photo restoration",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DenoiseLevel {
    Minus1,
    #[default]
    Zero,
    One,
    Two,
    Three,
}

impl DenoiseLevel {
    pub const ALL: [DenoiseLevel; 5] = [
        DenoiseLevel::Minus1,
        DenoiseLevel::Zero,
        DenoiseLevel::One,
        DenoiseLevel::Two,
        DenoiseLevel::Three,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DenoiseLevel::Minus1 => "-1",
            DenoiseLevel::Zero => "0",
            DenoiseLevel::One => "1",
            DenoiseLevel::Two => "2",
            DenoiseLevel::Three => "3",
        }
    }

    pub fn cli_value(self) -> &'static str {
        self.label()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum OutputFormat {
    #[default]
    Png,
    Jpg,
    Webp,
}

impl OutputFormat {
    pub fn ext(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Jpg => "jpg",
            OutputFormat::Webp => "webp",
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpscaleConfig {
    pub algorithm: Algorithm,
    pub variant: Variant,
    pub scale: u8,
    pub format: OutputFormat,
    pub denoise: DenoiseLevel,
    pub tta: bool,
    pub onnx_model: Option<PathBuf>,
}

impl UpscaleConfig {
    pub fn model_dir(&self, models_root: &Path) -> PathBuf {
        resolve_model_dir(models_root, self.variant)
    }
}

/// Unified layout uses `models/<subdir>/`; legacy Real-ESRGAN zip uses flat `models/`.
pub fn resolve_model_dir(models_root: &Path, variant: Variant) -> PathBuf {
    let nested = models_root.join(variant.model_subdir());
    if nested.is_dir() {
        return nested;
    }
    if matches!(variant, Variant::RealEsrgan(_)) && models_root.is_dir() {
        return models_root.to_path_buf();
    }
    nested
}

/// Apply algorithm defaults when switching backends.
pub fn on_algorithm_changed(
    algorithm: Algorithm,
    scale: &mut u8,
    variant: &mut Variant,
    denoise: &mut DenoiseLevel,
) {
    *variant = algorithm.default_variant();
    *scale = algorithm.clamp_scale(algorithm.default_scale());
    *denoise = algorithm.default_denoise();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn on_algorithm_change_resets_defaults() {
        let mut scale = 2;
        let mut variant = Variant::RealEsrgan(RealEsrganModel::AnimeV3);
        let mut denoise = DenoiseLevel::Three;
        on_algorithm_changed(Algorithm::RealCugan, &mut scale, &mut variant, &mut denoise);
        assert_eq!(scale, 4);
        assert_eq!(variant, Variant::RealCugan(RealCuganModel::Se));
        assert_eq!(denoise, DenoiseLevel::Minus1);
    }

    #[test]
    fn clamp_scale_realsr_only_4x() {
        assert_eq!(Algorithm::RealSr.clamp_scale(2), 4);
        assert_eq!(Algorithm::RealSr.clamp_scale(4), 4);
    }

    #[test]
    fn waifu2x_scales() {
        assert_eq!(Algorithm::Waifu2x.clamp_scale(3), 4);
        assert_eq!(Algorithm::Waifu2x.clamp_scale(2), 2);
    }

    #[test]
    fn variant_model_subdirs() {
        assert_eq!(
            Variant::Waifu2x(Waifu2xModel::Cunet).model_subdir(),
            "models-cunet"
        );
        assert_eq!(
            Variant::RealEsrgan(RealEsrganModel::X4Net).model_subdir(),
            "realesrgan"
        );
    }

    #[test]
    fn resolve_model_dir_legacy_flat_esrgan() {
        let tmp = tempfile::tempdir().unwrap();
        let models = tmp.path().join("models");
        fs::create_dir_all(&models).unwrap();
        fs::write(models.join("realesrgan-x4plus.param"), b"").unwrap();

        let got = resolve_model_dir(
            &models,
            Variant::RealEsrgan(RealEsrganModel::X4Plus),
        );
        assert_eq!(got, models);
    }

    #[test]
    fn resolve_model_dir_unified_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let models = tmp.path().join("models");
        let nested = models.join("realesrgan");
        fs::create_dir_all(&nested).unwrap();

        let got = resolve_model_dir(
            &models,
            Variant::RealEsrgan(RealEsrganModel::X4Plus),
        );
        assert_eq!(got, nested);
    }
}
