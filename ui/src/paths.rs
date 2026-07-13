use crate::models::Algorithm;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
    pub upscale_dir: PathBuf,
    pub backends: BackendPaths,
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
        let (upscale_dir, backends) = discover_backends(&root)?;

        if !backends.any_available() {
            return Err(
                "[ERROR: no upscaler binaries found] install under tools/upscale/".into(),
            );
        }

        Ok(Self {
            root,
            upscale_dir,
            backends,
        })
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

    pub fn exe_display(&self) -> String {
        let mut parts = Vec::new();
        for algo in Algorithm::ALL {
            if self.backends.get(algo).is_some() {
                parts.push(algo.header_label());
            }
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

fn discover_backends(root: &Path) -> Result<(PathBuf, BackendPaths), String> {
    let unified = root.join("tools/upscale");
    if unified.is_dir() {
        let models_root = unified.join("models");
        let backends = BackendPaths {
            realesrgan: resolve_backend(&unified, &models_root, Algorithm::RealEsrgan),
            realcugan: resolve_backend(&unified, &models_root, Algorithm::RealCugan),
            waifu2x: resolve_backend(&unified, &models_root, Algorithm::Waifu2x),
            realsr: resolve_backend(&unified, &models_root, Algorithm::RealSr),
        };
        return Ok((unified, backends));
    }

    // Legacy layout: tools/realesrgan-full/ with exe + models/ at that level.
    let legacy = root.join("tools/realesrgan-full");
    if legacy.is_dir() {
        let exe = legacy.join(Algorithm::RealEsrgan.exe_name());
        let models_root = legacy.join("models");
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

        let (dir, backends) = discover_backends(root).unwrap();
        assert_eq!(dir, legacy);
        assert!(backends.realesrgan.is_some());
        assert!(backends.realcugan.is_none());
    }

    #[test]
    fn unified_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let unified = root.join("tools/upscale");
        fs::create_dir_all(unified.join("models")).unwrap();
        for algo in Algorithm::ALL {
            fs::write(unified.join(algo.exe_name()), b"").unwrap();
        }

        let (_, backends) = discover_backends(root).unwrap();
        assert!(backends.realesrgan.is_some());
        assert!(backends.realcugan.is_some());
        assert!(backends.waifu2x.is_some());
        assert!(backends.realsr.is_some());
    }
}
