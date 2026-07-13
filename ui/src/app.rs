use crate::drop::{collect_paths, file_label, output_path_for};
use crate::icons::{Icon, Icons};
use crate::models::{
    on_algorithm_changed, Algorithm, DenoiseLevel, OutputFormat, UpscaleConfig, Variant,
};
use crate::paths::Paths;
use crate::preview::PreviewCache;
use crate::theme::{
    label_caps, progress_bar, run_button, segmented, setting_hint, setting_row, truncate_middle, NothingTheme,
    RunButtonState, SPACE_LG, SPACE_MD, SPACE_SM, SPACE_XL,
};
use crate::worker::{spawn_worker, WorkerHandle};
use eframe::egui::{self, Margin, Rect, Sense, Stroke, Vec2};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct QueueItem {
    pub input: PathBuf,
    pub output: PathBuf,
    pub done: bool,
    pub failed: bool,
}

pub enum RunState {
    Idle,
    Running,
    Done,
    Error(String),
}

pub struct UpscaleApp {
    paths: Result<Paths, String>,
    queue: Vec<QueueItem>,
    algorithm: Algorithm,
    variant: Variant,
    scale: u8,
    denoise: DenoiseLevel,
    tta: bool,
    format: OutputFormat,
    run_state: RunState,
    worker: Option<WorkerHandle>,
    drop_hovered: bool,
    status_message: Option<String>,
    preview_cache: PreviewCache,
    preview_idx: usize,
    was_processing: bool,
    icons: Icons,
}

impl UpscaleApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        NothingTheme::setup_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(NothingTheme::visuals());
        let icons = Icons::new(&cc.egui_ctx);

        Self {
            paths: Paths::discover(),
            queue: Vec::new(),
            algorithm: Algorithm::RealEsrgan,
            variant: Algorithm::RealEsrgan.default_variant(),
            scale: 4,
            denoise: DenoiseLevel::Zero,
            tta: false,
            format: OutputFormat::default(),
            run_state: RunState::Idle,
            worker: None,
            drop_hovered: false,
            status_message: None,
            preview_cache: PreviewCache::new(),
            preview_idx: 0,
            was_processing: false,
            icons,
        }
    }

    fn backend_available(&self) -> bool {
        self.paths
            .as_ref()
            .ok()
            .and_then(|p| p.require(self.algorithm).ok())
            .is_some()
    }

    fn can_run(&self) -> bool {
        matches!(
            self.run_state,
            RunState::Idle | RunState::Done | RunState::Error(_)
        ) && self.worker.as_ref().is_none_or(|w| !w.is_running())
            && self.backend_available()
            && self.queue.iter().any(|q| !q.done && !q.failed)
    }

    fn upscale_config(&self) -> UpscaleConfig {
        UpscaleConfig {
            algorithm: self.algorithm,
            variant: self.variant,
            scale: self.algorithm.clamp_scale(self.scale),
            format: self.format,
            denoise: self.denoise,
            tta: self.tta,
        }
    }

    fn available_algorithms(&self) -> Vec<(Algorithm, &'static str)> {
        let Some(paths) = self.paths.as_ref().ok() else {
            return Algorithm::ALL
                .iter()
                .map(|&a| (a, a.label()))
                .collect();
        };
        Algorithm::ALL
            .iter()
            .filter(|&&a| paths.backends.get(a).is_some())
            .map(|&a| (a, a.label()))
            .collect()
    }

    fn is_processing(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| w.is_running())
    }

    /// Real overall progress 0..1: finished files plus the current image's
    /// percentage (parsed from the backend), divided by the total file count.
    fn processing_progress(&self) -> Option<f32> {
        let worker = self.worker.as_ref()?;
        let prog = worker.progress();
        if !prog.running || prog.total == 0 {
            return None;
        }
        let total = prog.total.max(1) as f32;
        let completed = prog.current.saturating_sub(1) as f32;
        let image = (prog.image_percent / 100.0).clamp(0.0, 1.0);
        Some(((completed + image) / total).clamp(0.0, 1.0))
    }

    fn run_button_state(&self) -> RunButtonState {
        if self.is_processing() {
            RunButtonState::Processing
        } else if self.can_run() {
            RunButtonState::Ready
        } else {
            RunButtonState::Disabled
        }
    }

    fn ingest_path(&mut self, path: PathBuf) {
        let collected = collect_paths(&path);
        if collected.is_empty() {
            self.status_message = Some("[ERROR: unsupported file]".into());
            return;
        }
        for input in collected {
            if self.queue.iter().any(|q| q.input == input) {
                continue;
            }
            let output = output_path_for(&input, self.format);
            self.queue.push(QueueItem {
                input,
                output,
                done: false,
                failed: false,
            });
            self.preview_idx = self.queue.len() - 1;
        }
        self.status_message = None;
    }

    fn sync_queue_outputs(&mut self) {
        for item in &mut self.queue {
            if !item.done {
                item.output = output_path_for(&item.input, self.format);
            }
        }
    }

    fn open_file_dialog(&mut self) {
        if self.is_processing() {
            return;
        }
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("images", &["jpg", "jpeg", "png", "webp"])
            .set_title("SELECT IMAGES")
            .pick_files()
        {
            for path in paths {
                self.ingest_path(path);
            }
        }
    }

    fn start_run(&mut self) {
        let Ok(paths) = self.paths.clone() else {
            return;
        };

        let Ok(backend) = paths.require(self.algorithm).cloned() else {
            self.status_message = Some(format!(
                "[ERROR: {} not installed]",
                self.algorithm.header_label()
            ));
            return;
        };

        let pending: Vec<(PathBuf, PathBuf)> = self
            .queue
            .iter()
            .filter(|q| !q.done && !q.failed)
            .map(|q| (q.input.clone(), q.output.clone()))
            .collect();

        if pending.is_empty() {
            return;
        }

        self.run_state = RunState::Running;
        self.status_message = None;
        self.worker = Some(spawn_worker(
            backend,
            pending,
            self.upscale_config(),
        ));
    }

    fn poll_worker(&mut self) {
        let Some(worker) = &mut self.worker else {
            return;
        };

        worker.poll();

        if worker.is_running() {
            return;
        }

        let prog = worker.progress();

        if let Some(err) = prog.error {
            self.run_state = RunState::Error(err.clone());
            self.status_message = Some(err);
            self.worker = None;
            return;
        }

        if prog.finished {
            for item in &mut self.queue {
                if !item.done && !item.failed && item.output.is_file() {
                    item.done = true;
                }
            }
            self.run_state = RunState::Done;
            self.worker = None;
        }
    }

    fn preview_path(&self) -> Option<&Path> {
        let item = self.queue.get(self.preview_idx)?;
        if item.done && item.output.is_file() {
            Some(item.output.as_path())
        } else {
            Some(item.input.as_path())
        }
    }

    fn preview_label(&self) -> Option<(&'static str, String)> {
        let item = self.queue.get(self.preview_idx)?;
        let tag = if item.done && item.output.is_file() {
            "OUTPUT"
        } else {
            "INPUT"
        };
        Some((tag, file_label(&item.input)))
    }

    fn sync_preview_after_run(&mut self, ctx: &egui::Context) {
        let processing = self.is_processing();
        if self.was_processing && !processing {
            if let Some(item) = self.queue.get(self.preview_idx) {
                if item.output.is_file() {
                    self.preview_cache.reload(ctx, &item.output);
                }
            }
        }
        self.was_processing = processing;
    }

    fn status_text(&self) -> String {
        if let Some(worker) = &self.worker {
            let prog = worker.progress();
            if prog.running && prog.total > 0 {
                return format!(
                    "[{}/{}] {}",
                    prog.current.max(1),
                    prog.total,
                    truncate_middle(&prog.filename, 36)
                );
            }
        }
        match &self.run_state {
            RunState::Idle => "[IDLE]".into(),
            RunState::Running => "[PROCESSING]".into(),
            RunState::Done => "[DONE]".into(),
            RunState::Error(_) => "[ERROR]".into(),
        }
    }

    fn drop_zone_labels(&self) -> (String, String) {
        if self.drop_hovered {
            ("RELEASE TO ADD".into(), "jpg · png · webp".into())
        } else {
            ("DROP IMAGES".into(), "jpg · png · webp".into())
        }
    }

    fn draw_drop_zone(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let available = ui.available_size();
        // Reserve room for the queue (if shown), settings strip, CTA and footer,
        // then let the preview absorb whatever vertical space is left so the hero
        // grows with the window instead of leaving dead space.
        let queue_reserve = if self.queue.len() > 1 {
            40.0 + (self.queue.len().min(6) as f32) * 24.0
        } else {
            0.0
        };
        let denoise_reserve = if self.algorithm.supports_denoise() {
            44.0
        } else {
            0.0
        };
        let reserve = 490.0 + queue_reserve + denoise_reserve;
        let height = (available.y - reserve).clamp(200.0, (available.y * 0.72).max(200.0));
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(available.x, height), Sense::click_and_drag());

        let hover_target = if self.drop_hovered || response.hovered() {
            1.0
        } else {
            0.0
        };
        let hover_t = ctx.animate_value_with_time(response.id, hover_target, 0.18);
        let border =
            crate::theme::lerp_color(NothingTheme::BORDER, NothingTheme::BORDER_VISIBLE, hover_t);

        ui.painter().rect_filled(rect, 0.0, NothingTheme::SURFACE);
        ui.painter()
            .rect_stroke(rect, 0.0, Stroke::new(1.0, border));

        let preview_path = self.preview_path().map(|p| p.to_path_buf());
        let mut drew_preview = false;

        if let Some(path) = preview_path {
            if let Some(tex) = self.preview_cache.texture(ctx, &path) {
                let pad = 8.0;
                let inner = rect.shrink(pad);
                let tex_size = tex.size_vec2();
                let scale = (inner.width() / tex_size.x)
                    .min(inner.height() / tex_size.y)
                    .min(1.0);
                let display = tex_size * scale;
                let img_rect = Rect::from_center_size(inner.center(), display);
                ui.painter().image(
                    tex.id(),
                    img_rect,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                drew_preview = true;

                if let Some((tag, name)) = self.preview_label() {
                    let bar_h = 24.0;
                    let label_rect = Rect::from_min_max(
                        egui::pos2(rect.left(), rect.bottom() - bar_h),
                        egui::pos2(rect.right(), rect.bottom()),
                    );
                    ui.painter().rect_filled(
                        label_rect,
                        0.0,
                        NothingTheme::BLACK.gamma_multiply(0.85),
                    );
                    ui.painter().line_segment(
                        [label_rect.left_top(), label_rect.right_top()],
                        Stroke::new(1.0, NothingTheme::BORDER),
                    );
                    ui.painter().text(
                        label_rect.left_center() + Vec2::new(28.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        format!("{tag}  {}", truncate_middle(&name, 48)),
                        NothingTheme::font_label(),
                        NothingTheme::TEXT_SECONDARY,
                    );
                    self.icons.paint_at(
                        ui,
                        Icon::Image,
                        label_rect.left_center() + Vec2::new(12.0, 0.0),
                        12.0,
                        NothingTheme::TEXT_SECONDARY,
                    );
                }
            }
        }

        if !drew_preview {
            let (title, subtitle) = self.drop_zone_labels();
            let center = rect.center();
            let icon_color = crate::theme::lerp_color(
                NothingTheme::TEXT_SECONDARY,
                NothingTheme::TEXT_DISPLAY,
                hover_t,
            );
            self.icons.paint_at(
                ui,
                Icon::UploadSimple,
                center + Vec2::new(0.0, -48.0 - hover_t * 4.0),
                28.0 + hover_t * 2.0,
                icon_color,
            );
            ui.painter().text(
                center + Vec2::new(0.0, -8.0),
                egui::Align2::CENTER_CENTER,
                title,
                NothingTheme::font_display(),
                NothingTheme::TEXT_DISPLAY,
            );
            ui.painter().text(
                center + Vec2::new(0.0, 28.0),
                egui::Align2::CENTER_CENTER,
                subtitle,
                NothingTheme::font_body(),
                NothingTheme::TEXT_SECONDARY,
            );
        }

        // Hero scale readout — top-right, only when idle with a queue
        if !self.is_processing() && !self.queue.is_empty() {
            ui.painter().text(
                rect.right_top() + Vec2::new(-16.0, 16.0),
                egui::Align2::RIGHT_TOP,
                format!("{}×", self.scale),
                NothingTheme::font_hero(),
                NothingTheme::TEXT_DISPLAY,
            );
        }

        if response.clicked() && !self.is_processing() {
            self.open_file_dialog();
        }
    }

    fn draw_algorithm_picker(&mut self, ui: &mut egui::Ui, settings_enabled: bool) {
        let w = ui.available_width();
        let options = self.available_algorithms();
        if let Some(first) = options.first() {
            if !options.iter().any(|(a, _)| *a == self.algorithm) {
                self.algorithm = first.0;
                on_algorithm_changed(
                    self.algorithm,
                    &mut self.scale,
                    &mut self.variant,
                    &mut self.denoise,
                );
            }
            let prev = self.algorithm;
            segmented(ui, &mut self.algorithm, &options, settings_enabled, w);
            if self.algorithm != prev {
                on_algorithm_changed(
                    self.algorithm,
                    &mut self.scale,
                    &mut self.variant,
                    &mut self.denoise,
                );
            }
        }
    }

    fn draw_queue(&mut self, ui: &mut egui::Ui) {
        if self.queue.len() <= 1 {
            return;
        }
        ui.add_space(SPACE_MD);
        ui.horizontal(|ui| {
            self.icons.show(
                ui,
                Icon::Queue,
                NothingTheme::ICON_SIZE,
                NothingTheme::TEXT_SECONDARY,
            );
            ui.add_space(6.0);
            label_caps(ui, &format!("QUEUE · {}", self.queue.len()));
        });
        ui.add_space(SPACE_SM);
        egui::Frame::none()
            .fill(NothingTheme::SURFACE_RAISED)
            .stroke(Stroke::new(1.0, NothingTheme::BORDER))
            .inner_margin(Margin::symmetric(12.0, 6.0))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                for (idx, item) in self.queue.iter().enumerate() {
                    let selected = idx == self.preview_idx;
                    let name_color = if item.failed {
                        NothingTheme::ACCENT
                    } else if item.done {
                        NothingTheme::TEXT_DISABLED
                    } else if selected {
                        NothingTheme::TEXT_DISPLAY
                    } else {
                        NothingTheme::TEXT_PRIMARY
                    };

                    ui.horizontal(|ui| {
                        let marker = if selected { "›" } else { " " };
                        ui.label(
                            egui::RichText::new(marker)
                                .font(NothingTheme::font_label())
                                .color(NothingTheme::TEXT_DISABLED),
                        );
                        let resp = ui.selectable_label(
                            false,
                            egui::RichText::new(truncate_middle(&file_label(&item.input), 32))
                                .font(NothingTheme::font_body())
                                .color(name_color)
                                .family(egui::FontFamily::Monospace),
                        );
                        ui.label(
                            egui::RichText::new("→")
                                .font(NothingTheme::font_label())
                                .color(NothingTheme::TEXT_DISABLED),
                        );
                        ui.label(
                            egui::RichText::new(truncate_middle(&file_label(&item.output), 32))
                                .font(NothingTheme::font_body())
                                .color(NothingTheme::TEXT_DISABLED)
                                .family(egui::FontFamily::Monospace),
                        );
                        if resp.clicked() {
                            self.preview_idx = idx;
                        }
                    });
                }
            });
    }
}

impl eframe::App for UpscaleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.is_processing() {
            let mut dropped = Vec::new();
            let mut hovered = false;
            ctx.input(|i| {
                hovered = !i.raw.hovered_files.is_empty();
                for file in &i.raw.dropped_files {
                    if let Some(path) = &file.path {
                        dropped.push(path.clone());
                    }
                }
            });
            self.drop_hovered = hovered;
            for path in dropped {
                self.ingest_path(path);
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && self.can_run() {
            self.start_run();
        }

        self.poll_worker();
        self.sync_preview_after_run(ctx);

        if let Some(worker) = &self.worker {
            let prog = worker.progress();
            if prog.running {
                if let Some(idx) = self
                    .queue
                    .iter()
                    .position(|q| file_label(&q.input) == prog.filename)
                {
                    self.preview_idx = idx;
                }
            }
        }

        let settings_enabled = !self.is_processing();
        let progress = self.processing_progress();
        let time = ctx.input(|i| i.time as f32);
        let spin = time * 4.0;

        if self.is_processing() {
            ctx.request_repaint();
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(NothingTheme::BLACK)
                    .inner_margin(Margin::symmetric(24.0, 20.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), 32.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            self.icons.show(
                                ui,
                                Icon::MagicWand,
                                24.0,
                                NothingTheme::TEXT_DISPLAY,
                            );
                            ui.add_space(SPACE_SM);
                            ui.label(
                                egui::RichText::new("Loku")
                                    .font(NothingTheme::font_display())
                                    .color(NothingTheme::TEXT_DISPLAY)
                                    .size(24.0),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    self.icons.show(
                                        ui,
                                        Icon::Cpu,
                                        NothingTheme::ICON_SIZE,
                                        NothingTheme::TEXT_DISABLED,
                                    );
                                    ui.add_space(6.0);
                                    label_caps(ui, self.algorithm.header_label());
                                },
                            );
                        },
                    );
                });

                ui.add_space(SPACE_LG);
                self.draw_drop_zone(ui, ctx);
                self.draw_queue(ui);

                ui.add_space(SPACE_XL);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.allocate_ui_with_layout(
                        Vec2::new(crate::theme::LABEL_COL, 44.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            self.icons.show(
                                ui,
                                Icon::Cpu,
                                NothingTheme::ICON_SIZE,
                                NothingTheme::TEXT_SECONDARY,
                            );
                            ui.add_space(6.0);
                            label_caps(ui, "ENGINE");
                        },
                    );
                    self.draw_algorithm_picker(ui, settings_enabled);
                });
                setting_hint(ui, self.algorithm.description());

                ui.add_space(SPACE_SM);
                setting_row(ui, &self.icons, Icon::Cpu, "MODEL", |ui| {
                    let w = ui.available_width();
                    let options = self.algorithm.variant_options();
                    let mut picked = self.variant;
                    segmented(ui, &mut picked, options, settings_enabled, w);
                    self.variant = picked;
                });
                setting_hint(ui, self.variant.description());

                ui.add_space(SPACE_SM);
                setting_row(ui, &self.icons, Icon::ArrowsOut, "SCALE", |ui| {
                    let w = ui.available_width();
                    let scales: Vec<(u8, String)> = self
                        .algorithm
                        .valid_scales()
                        .iter()
                        .map(|&s| (s, s.to_string()))
                        .collect();
                    let scale_opts: Vec<(u8, &str)> = scales
                        .iter()
                        .map(|(v, l)| (*v, l.as_str()))
                        .collect();
                    let mut scale = self.algorithm.clamp_scale(self.scale);
                    segmented(ui, &mut scale, &scale_opts, settings_enabled, w);
                    self.scale = scale;
                });

                if self.algorithm.supports_denoise() {
                    ui.add_space(SPACE_SM);
                    setting_row(ui, &self.icons, Icon::Sparkle, "DENOISE", |ui| {
                        let w = ui.available_width();
                        let opts: Vec<(DenoiseLevel, &str)> = DenoiseLevel::ALL
                            .iter()
                            .map(|&d| (d, d.label()))
                            .collect();
                        segmented(ui, &mut self.denoise, &opts, settings_enabled, w);
                    });
                }

                ui.add_space(SPACE_SM);
                setting_row(ui, &self.icons, Icon::Sparkle, "TTA", |ui| {
                    let w = ui.available_width();
                    let mut tta_on = self.tta;
                    segmented(
                        ui,
                        &mut tta_on,
                        &[(false, "OFF"), (true, "ON")],
                        settings_enabled,
                        w,
                    );
                    self.tta = tta_on;
                });

                ui.add_space(SPACE_SM);
                let prev_format = self.format;
                setting_row(ui, &self.icons, Icon::FilePng, "FORMAT", |ui| {
                    let w = ui.available_width();
                    segmented(
                        ui,
                        &mut self.format,
                        &[
                            (OutputFormat::Png, "PNG"),
                            (OutputFormat::Jpg, "JPG"),
                            (OutputFormat::Webp, "WEBP"),
                        ],
                        settings_enabled,
                        w,
                    );
                });
                if self.format != prev_format {
                    self.sync_queue_outputs();
                }

                ui.add_space(SPACE_XL);
                let cta_width = ui.available_width();
                if let Some(target) = progress {
                    // Ease the displayed value so tile-by-tile jumps glide.
                    let frac =
                        ctx.animate_value_with_time(egui::Id::new("loku_progress"), target, 0.25);
                    progress_bar(ui, frac, cta_width);
                    ui.add_space(SPACE_MD);
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
                        self.icons.paint_at_rotated(
                            ui,
                            Icon::CircleNotch,
                            icon_rect.center(),
                            14.0,
                            NothingTheme::TEXT_SECONDARY,
                            spin,
                        );
                        ui.add_space(SPACE_SM);
                        ui.label(
                            egui::RichText::new(format!(
                                "UPSCALING · {}%",
                                (frac * 100.0).round() as u32
                            ))
                            .font(NothingTheme::font_label())
                            .color(NothingTheme::TEXT_SECONDARY),
                        );
                    });
                } else if run_button(ui, &self.icons, self.run_button_state(), cta_width).clicked()
                {
                    self.start_run();
                }

                let error_text = self.status_message.clone().or_else(|| {
                    if let RunState::Error(err) = &self.run_state {
                        Some(err.clone())
                    } else {
                        None
                    }
                });
                if let Some(err) = error_text {
                    ui.add_space(SPACE_MD);
                    ui.horizontal(|ui| {
                        let (icon_rect, _) = ui.allocate_exact_size(
                            Vec2::splat(NothingTheme::ICON_SIZE),
                            egui::Sense::hover(),
                        );
                        self.icons.paint_at(
                            ui,
                            Icon::Warning,
                            icon_rect.center(),
                            NothingTheme::ICON_SIZE,
                            NothingTheme::ACCENT,
                        );
                        ui.add_space(SPACE_SM);
                        ui.label(
                            egui::RichText::new(err)
                                .font(NothingTheme::font_body())
                                .color(NothingTheme::ACCENT),
                        );
                    });
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    let exe_hint = match &self.paths {
                        Ok(p) => p.exe_display(),
                        Err(e) => e.clone(),
                    };
                    ui.horizontal(|ui| {
                        let status_icon = if self.is_processing() {
                            Icon::CircleNotch
                        } else if matches!(self.run_state, RunState::Done) {
                            Icon::Check
                        } else if self.status_message.is_some()
                            || matches!(self.run_state, RunState::Error(_))
                        {
                            Icon::Warning
                        } else {
                            Icon::Sparkle
                        };
                        let tint = if self.status_message.is_some()
                            || matches!(self.run_state, RunState::Error(_))
                        {
                            NothingTheme::ACCENT
                        } else {
                            NothingTheme::TEXT_SECONDARY
                        };
                        let (icon_rect, _) = ui.allocate_exact_size(
                            Vec2::splat(NothingTheme::ICON_SIZE),
                            egui::Sense::hover(),
                        );
                        self.icons.paint_at_rotated(
                            ui,
                            status_icon,
                            icon_rect.center(),
                            NothingTheme::ICON_SIZE,
                            tint,
                            if self.is_processing() { spin } else { 0.0 },
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(self.status_text())
                                .font(NothingTheme::font_label())
                                .color(tint),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(exe_hint)
                                    .font(NothingTheme::font_label())
                                    .color(NothingTheme::TEXT_DISABLED),
                            );
                        });
                    });
                });
            });

        if self.is_processing() || self.drop_hovered {
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Dropping the worker cancels it and kills the live backend process so
        // it doesn't keep running after the window closes.
        self.worker = None;
    }
}
