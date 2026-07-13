use crate::drop::{gpu_id, tile_size_for};
use crate::models::{Algorithm, UpscaleConfig};
use crate::paths::Backend;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone, Debug, Default)]
pub struct WorkerProgress {
    pub current: usize,
    pub total: usize,
    pub filename: String,
    /// Real per-image progress (0–100) parsed from the backend's stderr.
    pub image_percent: f32,
    pub running: bool,
    pub finished: bool,
    pub error: Option<String>,
}

/// ncnn-vulkan backends print lines like `12.50%` to stderr as each tile finishes.
fn parse_percent(line: &str) -> Option<f32> {
    let t = line.trim().strip_suffix('%')?;
    t.trim().parse::<f32>().ok()
}

pub struct WorkerHandle {
    progress: Arc<Mutex<WorkerProgress>>,
    cancel: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    join: Option<thread::JoinHandle<()>>,
}

impl WorkerHandle {
    pub fn progress(&self) -> WorkerProgress {
        self.progress.lock().unwrap().clone()
    }

    pub fn is_running(&self) -> bool {
        self.join.is_some()
    }

    pub fn poll(&mut self) {
        if let Some(handle) = self.join.take() {
            if handle.is_finished() {
                let _ = handle.join();
            } else {
                self.join = Some(handle);
            }
        }
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub fn build_cli_args(
    backend: &Backend,
    config: &UpscaleConfig,
    input: &Path,
    output: &Path,
    tile: &str,
    gpu: &str,
) -> Vec<String> {
    let model_dir = config.model_dir(&backend.models_root);
    let scale = config.algorithm.clamp_scale(config.scale).to_string();

    let mut args = vec![
        "-i".into(),
        input.to_string_lossy().into_owned(),
        "-o".into(),
        output.to_string_lossy().into_owned(),
    ];

    match config.algorithm {
        Algorithm::RealEsrgan => {
            if let Some(name) = config.variant.esrgan_cli_name() {
                args.extend(["-n".into(), name.into()]);
            }
            args.extend([
                "-s".into(),
                scale,
                "-f".into(),
                config.format.ext().into(),
                "-m".into(),
                model_dir.to_string_lossy().into_owned(),
            ]);
        }
        Algorithm::RealCugan | Algorithm::Waifu2x => {
            args.extend([
                "-s".into(),
                scale,
                "-n".into(),
                config.denoise.cli_value().into(),
                "-f".into(),
                config.format.ext().into(),
                "-m".into(),
                model_dir.to_string_lossy().into_owned(),
            ]);
        }
        Algorithm::RealSr => {
            args.extend([
                "-s".into(),
                scale,
                "-f".into(),
                config.format.ext().into(),
                "-m".into(),
                model_dir.to_string_lossy().into_owned(),
            ]);
        }
    }

    args.extend([
        "-t".into(),
        tile.into(),
        "-j".into(),
        "1:1:1".into(),
        "-g".into(),
        gpu.into(),
    ]);

    if config.tta {
        args.push("-x".into());
    }

    args
}

pub fn spawn_worker(
    backend: Backend,
    items: Vec<(PathBuf, PathBuf)>,
    config: UpscaleConfig,
) -> WorkerHandle {
    let progress = Arc::new(Mutex::new(WorkerProgress {
        total: items.len(),
        ..Default::default()
    }));
    let cancel = Arc::new(AtomicBool::new(false));
    let child_slot: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));

    let progress_clone = Arc::clone(&progress);
    let cancel_clone = Arc::clone(&cancel);
    let child_clone = Arc::clone(&child_slot);
    let join = thread::spawn(move || {
        let mut prog = progress_clone.lock().unwrap();
        prog.running = true;
        prog.total = items.len();
        drop(prog);

        for (index, (input, output)) in items.into_iter().enumerate() {
            if cancel_clone.load(Ordering::SeqCst) {
                break;
            }
            {
                let mut prog = progress_clone.lock().unwrap();
                prog.current = index + 1;
                prog.image_percent = 0.0;
                prog.filename = input
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
            }

            let tile = tile_size_for(&input, config.algorithm.clamp_scale(config.scale))
                .to_string();
            let gpu = gpu_id();
            let args = build_cli_args(&backend, &config, &input, &output, &tile, &gpu);

            let spawned = Command::new(&backend.exe)
                .args(&args)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();

            let mut child = match spawned {
                Ok(c) => c,
                Err(e) => {
                    let mut prog = progress_clone.lock().unwrap();
                    prog.running = false;
                    prog.finished = true;
                    prog.error = Some(format!("[ERROR: {e}]"));
                    return;
                }
            };

            let stderr = child.stderr.take();
            *child_clone.lock().unwrap() = Some(child);
            if cancel_clone.load(Ordering::SeqCst) {
                if let Some(mut c) = child_clone.lock().unwrap().take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                return;
            }

            if let Some(stderr) = stderr {
                let mut reader = std::io::BufReader::new(stderr);
                let mut line = Vec::new();
                let mut byte = [0u8; 1];
                while let Ok(1) = reader.read(&mut byte) {
                    if byte[0] == b'\r' || byte[0] == b'\n' {
                        if !line.is_empty() {
                            let text = String::from_utf8_lossy(&line);
                            if let Some(pct) = parse_percent(&text) {
                                progress_clone.lock().unwrap().image_percent = pct;
                            }
                            line.clear();
                        }
                    } else {
                        line.push(byte[0]);
                    }
                }
            }

            let Some(mut child) = child_clone.lock().unwrap().take() else {
                return;
            };

            match child.wait() {
                Ok(s) if s.success() => {
                    progress_clone.lock().unwrap().image_percent = 100.0;
                }
                Ok(s) => {
                    let mut prog = progress_clone.lock().unwrap();
                    prog.running = false;
                    prog.finished = true;
                    prog.error = Some(format!(
                        "[ERROR: process exited {}] {}",
                        s.code().unwrap_or(-1),
                        input.display()
                    ));
                    return;
                }
                Err(e) => {
                    let mut prog = progress_clone.lock().unwrap();
                    prog.running = false;
                    prog.finished = true;
                    prog.error = Some(format!("[ERROR: {e}]"));
                    return;
                }
            }
        }

        let mut prog = progress_clone.lock().unwrap();
        prog.running = false;
        prog.finished = true;
    });

    WorkerHandle {
        progress,
        cancel,
        child: child_slot,
        join: Some(join),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Algorithm, RealCuganModel, RealEsrganModel, RealSrModel, Waifu2xModel,
    };
    use crate::paths::Backend;
    use std::path::PathBuf;

    fn test_backend() -> Backend {
        Backend {
            exe: PathBuf::from("/tmp/upscale/realesrgan-ncnn-vulkan"),
            models_root: PathBuf::from("/tmp/upscale/models"),
        }
    }

    #[test]
    fn esrgan_args_include_model_name() {
        let backend = test_backend();
        let config = UpscaleConfig {
            algorithm: Algorithm::RealEsrgan,
            variant: Variant::RealEsrgan(RealEsrganModel::X4Net),
            scale: 4,
            format: OutputFormat::Png,
            denoise: DenoiseLevel::Zero,
            tta: false,
        };
        let args = build_cli_args(
            &backend,
            &config,
            Path::new("/in.jpg"),
            Path::new("/out.png"),
            "256",
            "0",
        );
        assert!(args.contains(&"-n".to_string()));
        assert!(args.contains(&"realesrnet-x4plus".to_string()));
        assert!(args.contains(&"-m".to_string()));
        assert!(args.iter().any(|a| a.contains("realesrgan")));
    }

    #[test]
    fn waifu2x_args_include_denoise() {
        let backend = test_backend();
        let config = UpscaleConfig {
            algorithm: Algorithm::Waifu2x,
            variant: Variant::Waifu2x(Waifu2xModel::Cunet),
            scale: 2,
            format: OutputFormat::Webp,
            denoise: DenoiseLevel::One,
            tta: true,
        };
        let args = build_cli_args(
            &backend,
            &config,
            Path::new("/in.jpg"),
            Path::new("/out.webp"),
            "512",
            "1",
        );
        assert!(args.contains(&"-n".to_string()));
        assert!(args.contains(&"1".to_string()));
        assert!(args.contains(&"-x".to_string()));
        assert!(args.iter().any(|a| a.contains("models-cunet")));
    }

    #[test]
    fn realsr_forces_4x_scale() {
        let backend = test_backend();
        let config = UpscaleConfig {
            algorithm: Algorithm::RealSr,
            variant: Variant::RealSr(RealSrModel::Df2k),
            scale: 2,
            format: OutputFormat::Jpg,
            denoise: DenoiseLevel::Zero,
            tta: false,
        };
        let args = build_cli_args(
            &backend,
            &config,
            Path::new("/in.jpg"),
            Path::new("/out.jpg"),
            "256",
            "0",
        );
        let s_idx = args.iter().position(|a| a == "-s").unwrap();
        assert_eq!(args[s_idx + 1], "4");
    }

    #[test]
    fn cugan_args() {
        let backend = test_backend();
        let config = UpscaleConfig {
            algorithm: Algorithm::RealCugan,
            variant: Variant::RealCugan(RealCuganModel::Se),
            scale: 3,
            format: OutputFormat::Png,
            denoise: DenoiseLevel::Minus1,
            tta: false,
        };
        let args = build_cli_args(
            &backend,
            &config,
            Path::new("/in.jpg"),
            Path::new("/out.png"),
            "384",
            "0",
        );
        assert!(args.contains(&"-n".to_string()));
        assert!(args.contains(&"-1".to_string()));
        assert!(args.iter().any(|a| a.contains("models-se")));
    }
}
