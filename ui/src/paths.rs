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
    pub suggest_models: Option<suggest::SuggestModels>,
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
        let (upscale_dir, backends, onnx_models, suggest_models) = discover_backends(&root)?;

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
            suggest_models,
        })
    }

    pub fn suggest_available(&self) -> bool {
        self.suggest_models.is_some()
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
}

fn discover_backends(
    root: &Path,
) -> Result<
    (
        PathBuf,
        BackendPaths,
        Vec<PathBuf>,
        Option<suggest::SuggestModels>,
    ),
    String,
> {
    let unified = root.join("tools/upscale");
    if unified.is_dir() {
        let models_root = unified.join("models");
        let onnx_models = onnx::discover_models(&models_root);
        let suggest_models = suggest::discover_models(&models_root);
        let backends = BackendPaths {
            realesrgan: resolve_backend(&unified, &models_root, Algorithm::RealEsrgan),
            realcugan: resolve_backend(&unified, &models_root, Algorithm::RealCugan),
            waifu2x: resolve_backend(&unified, &models_root, Algorithm::Waifu2x),
            realsr: resolve_backend(&unified, &models_root, Algorithm::RealSr),
        };
        return Ok((unified, backends, onnx_models, suggest_models));
    }

    // Legacy layout: tools/realesrgan-full/ with exe + models/ at that level.
    let legacy = root.join("tools/realesrgan-full");
    if legacy.is_dir() {
        let exe = legacy.join(Algorithm::RealEsrgan.exe_name());
        let models_root = legacy.join("models");
        let onnx_models = onnx::discover_models(&models_root);
        let suggest_models = suggest::discover_models(&models_root);
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
            suggest_models,
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

/// User data root for engines when Loku is installed outside the git checkout
/// (`%LOCALAPPDATA%\loku` on Windows, Application Support / XDG elsewhere).
pub fn data_home() -> PathBuf {
    if let Ok(override_root) = env::var("LOKU_DATA") {
        return PathBuf::from(override_root);
    }
    if cfg!(windows) {
        if let Ok(local) = env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("loku");
        }
    }
    if cfg!(target_os = "macos") {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join("Library/Application Support/loku");
        }
    }
    if let Ok(xdg) = env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("loku");
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".local/share/loku");
    }
    PathBuf::from("loku")
}

fn looks_like_root(dir: &Path) -> bool {
    dir.join("tools/upscale").is_dir()
        || dir.join("tools/realesrgan-full").is_dir()
        || (dir.join("ui/Cargo.toml").is_file() && dir.join("tools").is_dir())
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
            if looks_like_root(dir) {
                return Ok(dir.to_path_buf());
            }
            dir = match dir.parent() {
                Some(p) => p,
                None => break,
            };
        }
    }

    let data = data_home();
    if looks_like_root(&data) {
        return Ok(data);
    }

    Err(format!(
        "[ERROR: could not find engines] run tools/setup.ps1 (or setup.sh), or set UPSCALE_ROOT — expected tools/upscale under the repo or {}",
        data.display()
    ))
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
    fn suggest_models_discovered() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let unified = root.join("tools/upscale");
        let suggest_dir = unified.join("models/suggest");
        fs::create_dir_all(&suggest_dir).unwrap();
        fs::write(unified.join(Algorithm::RealEsrgan.exe_name()), b"").unwrap();
        let anime_real = suggest_dir.join("anime_real.onnx");
        let anime_cls = suggest_dir.join("anime_cls.onnx");
        fs::write(&anime_real, b"fake").unwrap();
        fs::write(&anime_cls, b"fake").unwrap();

        let (_, _, _, suggest) = discover_backends(root).unwrap();
        let suggest = suggest.expect("suggest models");
        assert_eq!(suggest.anime_real, anime_real);
        assert_eq!(suggest.anime_cls, anime_cls);
    }

    #[test]
    fn looks_like_root_accepts_tools_upscale() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!looks_like_root(tmp.path()));
        fs::create_dir_all(tmp.path().join("tools/upscale")).unwrap();
        assert!(looks_like_root(tmp.path()));
    }
}
