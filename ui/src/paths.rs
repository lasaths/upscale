use std::env;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Paths {
    pub root: PathBuf,
    pub exe: PathBuf,
    pub models: PathBuf,
}

impl Paths {
    pub fn discover() -> Result<Self, String> {
        let root = find_root()?;
        let exe = root.join("tools/realesrgan-full/realesrgan-ncnn-vulkan.exe");
        let models = root.join("tools/realesrgan-full/models");

        if !exe.is_file() {
            return Err(format!("[ERROR: binary not found] {}", exe.display()));
        }
        if !models.is_dir() {
            return Err(format!("[ERROR: models not found] {}", models.display()));
        }

        Ok(Self { root, exe, models })
    }

    pub fn exe_display(&self) -> String {
        shorten(&self.exe, &self.root)
    }
}

fn find_root() -> Result<PathBuf, String> {
    if let Ok(env_root) = env::var("UPSCALE_ROOT") {
        let p = PathBuf::from(env_root);
        if p.join("tools/realesrgan-full").is_dir() {
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
            if dir.join("tools/realesrgan-full").is_dir() {
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
