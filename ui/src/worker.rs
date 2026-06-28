use crate::drop::{gpu_id, tile_size_for};
use crate::models::{Model, OutputFormat};
use std::io::Read;
use std::path::PathBuf;
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

/// realesrgan-ncnn-vulkan prints lines like `12.50%` to stderr as each tile
/// finishes. Pull the trailing percentage out of one such line.
fn parse_percent(line: &str) -> Option<f32> {
    let t = line.trim().strip_suffix('%')?;
    t.trim().parse::<f32>().ok()
}

pub struct WorkerHandle {
    progress: Arc<Mutex<WorkerProgress>>,
    cancel: Arc<AtomicBool>,
    /// The currently-running backend process, so we can kill it on shutdown
    /// instead of orphaning it when the window closes.
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
        // Kill the live backend process so it doesn't outlive the app. This also
        // unblocks the worker thread's stderr read so it can exit promptly.
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub fn spawn_worker(
    exe: PathBuf,
    models: PathBuf,
    items: Vec<(PathBuf, PathBuf)>,
    model: Model,
    scale: u8,
    format: OutputFormat,
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

            let tile = tile_size_for(&input, scale).to_string();
            let gpu = gpu_id();

            let spawned = Command::new(&exe)
                .args([
                    "-i",
                    &input.to_string_lossy(),
                    "-o",
                    &output.to_string_lossy(),
                    "-n",
                    model.cli_name(),
                    "-s",
                    &scale.to_string(),
                    "-f",
                    format.ext(),
                    "-m",
                    &models.to_string_lossy(),
                    "-t",
                    &tile,
                    "-j",
                    "1:1:1",
                    "-g",
                    &gpu,
                ])
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
            // Publish the live child so a shutdown can kill it. If cancellation
            // already fired in the spawn window, kill it right away.
            *child_clone.lock().unwrap() = Some(child);
            if cancel_clone.load(Ordering::SeqCst) {
                if let Some(mut c) = child_clone.lock().unwrap().take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                return;
            }

            // Stream stderr; progress is emitted with `\r`, so split on both.
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

            // Reclaim the child to wait on it. If a shutdown already took and
            // killed it, the slot is empty — stop quietly.
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
