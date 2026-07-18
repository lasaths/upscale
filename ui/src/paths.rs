use crate::models::Algorithm;
use crate::onnx;
use crate::suggest;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
    pub upscale_dir: PathBuf,
    pub backends: BackendPaths,
    pub onnx_models: Vec<PathBuf>,
    pub suggest_model: Option<PathBuf>,
}

#[derive(Clone, Debug, Default)]
pub struct BackendPaths {
    pub realesrgan: Option<Backend>,
    pub realcugan: Option<Backend>,
    pub waifu2x: Option<Backend>,
    pub realsr: Option<Backend>,
}

#[derive(Clone, Debug)]
pub struct Backend {
    pub exe: PathBuf,
    pub models_root: PathBuf,
}

impl BackendPaths {
    pub fn get(&self, algorithm: Algorithm) -> Option<&Backend> {
        match algorithm {
            Algorithm::RealEsrgan => self.realesrgan.as_ref(),
            Algorithm::RealCugan => self.realcugan.as_ref(),
            Algorithm::Waifu2x => self.waifu2x.as_ref(),
            Algorithm::RealSr => self.realsr.as_ref(),
            Algorithm::Onnx => None,
        }
    }

    pub fn any_available(&self) -> bool {
        self.realesrgan.is_some()
            || self.realcugan.is_some()
            || self.waifu2x.is_some()
            || self.realsr.is_some()
    }
}

impl Paths {
    pub fn discover() -> Result<Self, String> {
        let root = find_root()?;
        let (upscale_dir, backends, onnx_models, suggest_model) = discover_backends(&root)?;

        if !backends.any_available() && onnx_models.is_empty() {
            return Err(
                "[ERROR: no upscaler found] install ncnn binaries or ONNX models under tools/upscale/"
                    .into(),
            );
        }

        Ok(Self {
            root,
            upscale_dir,
            backends,
            onnx_models,
            suggest_model,
        })
    }

    pub fn suggest_available(&self) -> bool {
        self.suggest_model.is_some()
    }

    pub fn require(&self, algorithm: Algorithm) -> Result<&Backend, String> {
        self.backends.get(algorithm).ok_or_else(|| {
            format!(
                "[ERROR: {} not found] install {} under tools/upscale/",
                algorithm.header_label(),
                algorithm.exe_name()
            )
        })
    }

    pub fn onnx_available(&self) -> bool {
        !self.onnx_models.is_empty()
    }

    pub fn exe_display(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for algo in Algorithm::NCNN {
            if self.backends.get(algo).is_some() {
                parts.push(algo.header_label().into());
            }
        }
        if self.onnx_available() {
            parts.push(format!("onnx ({})", self.onnx_models.len()));
        }
        if parts.is_empty() {
            shorten(&self.upscale_dir, &self.root)
        } else if self.upscale_dir.ends_with("realesrgan-full") {
            format!("tools/realesrgan-full · {}", parts.join(", "))
        } else {
            format!("tools/upscale · {}", parts.join(", "))
        }
    }
}

fn discover_backends(
    root: &Path,
) -> Result<(PathBuf, BackendPaths, Vec<PathBuf>, Option<PathBuf>), String> {
    let unified = root.join("tools/upscale");
    if unified.is_dir() {
        let models_root = unified.join("models");
        let onnx_models = onnx::discover_models(&models_root);
        let suggest_model = suggest::discover_model(&models_root);
        let backends = BackendPaths {
            realesrgan: resolve_backend(&unified, &models_root, Algorithm::RealEsrgan),
            realcugan: resolve_backend(&unified, &models_root, Algorithm::RealCugan),
            waifu2x: resolve_backend(&unified, &models_root, Algorithm::Waifu2x),
            realsr: resolve_backend(&unified, &models_root, Algorithm::RealSr),
        };
        return Ok((unified, backends, onnx_models, suggest_model));
    }

    // Legacy layout: tools/realesrgan-full/ with exe + models/ at that level.
    let legacy = root.join("tools/realesrgan-full");
    if legacy.is_dir() {
        let exe = legacy.join(Algorithm::RealEsrgan.exe_name());
        let models_root = legacy.join("models");
        let onnx_models = onnx::discover_models(&models_root);
        let suggest_model = suggest::discover_model(&models_root);
        let realesrgan = if exe.is_file() {
            Some(Backend {
                exe,
                models_root: models_root.clone(),
            })
        } else {
            None
        };
        return Ok((
            legacy.clone(),
            BackendPaths {
                realesrgan,
                ..Default::default()
            },
            onnx_models,
            suggest_model,
        ));
    }

    Err("[ERROR: could not find tools/upscale or tools/realesrgan-full] set UPSCALE_ROOT".into())
}

fn resolve_backend(
    upscale_dir: &Path,
    models_root: &Path,
    algorithm: Algorithm,
) -> Option<Backend> {
    let exe = upscale_dir.join(algorithm.exe_name());
    if !exe.is_file() {
        return None;
    }
    Some(Backend {
        exe,
        models_root: models_root.to_path_buf(),
    })
}

fn find_root() -> Result<PathBuf, String> {
    if let Ok(env_root) = env::var("UPSCALE_ROOT") {
        let p = PathBuf::from(env_root);
        if p.is_dir() {
            return Ok(p);
        }
    }

    let mut candidates = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.to_path_buf());
        }
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd);
    }

    for start in candidates {
        let mut dir = start.as_path();
        loop {
            if dir.join("ui/Cargo.toml").is_file() && dir.join("tools").is_dir() {
                return Ok(dir.to_path_buf());
            }
            dir = match dir.parent() {
                Some(p) => p,
                None => break,
            };
        }
    }

    Err("[ERROR: could not find repo root] set UPSCALE_ROOT".into())
}

fn shorten(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn legacy_realesrgan_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let legacy = root.join("tools/realesrgan-full");
        fs::create_dir_all(legacy.join("models")).unwrap();
        let exe_name = Algorithm::RealEsrgan.exe_name();
        fs::write(legacy.join(exe_name), b"").unwrap();

        let (dir, backends, onnx, suggest) = discover_backends(root).unwrap();
        assert_eq!(dir, legacy);
        assert!(backends.realesrgan.is_some());
        assert!(backends.realcugan.is_none());
        assert!(onnx.is_empty());
        assert!(suggest.is_none());
    }

    #[test]
    fn unified_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let unified = root.join("tools/upscale");
        fs::create_dir_all(unified.join("models")).unwrap();
        for algo in Algorithm::NCNN {
            fs::write(unified.join(algo.exe_name()), b"").unwrap();
        }

        let (_, backends, onnx, suggest) = discover_backends(root).unwrap();
        assert!(backends.realesrgan.is_some());
        assert!(backends.realcugan.is_some());
        assert!(backends.waifu2x.is_some());
        assert!(backends.realsr.is_some());
        assert!(onnx.is_empty());
        assert!(suggest.is_none());
    }

    #[test]
    fn onnx_only_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let unified = root.join("tools/upscale");
        let onnx_dir = unified.join("models/onnx");
        fs::create_dir_all(&onnx_dir).unwrap();
        fs::write(onnx_dir.join("test.onnx"), b"fake").unwrap();

        let (_, backends, onnx, _) = discover_backends(root).unwrap();
        assert!(!backends.any_available());
        assert_eq!(onnx.len(), 1);
    }

    #[test]
    fn suggest_model_discovered() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let unified = root.join("tools/upscale");
        let suggest_dir = unified.join("models/suggest");
        fs::create_dir_all(&suggest_dir).unwrap();
        fs::write(unified.join(Algorithm::RealEsrgan.exe_name()), b"").unwrap();
        let model = suggest_dir.join("medium_classify.onnx");
        fs::write(&model, b"fake").unwrap();

        let (_, _, _, suggest) = discover_backends(root).unwrap();
        assert_eq!(suggest.as_deref(), Some(model.as_path()));
    }
}
