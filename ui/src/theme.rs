use crate::icons::Icon;
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, Rounding, Stroke, Vec2,
    Visuals,
};

pub struct NothingTheme;

pub const LABEL_COL: f32 = 88.0;

pub const SPACE_SM: f32 = 8.0;
pub const SPACE_MD: f32 = 12.0;
pub const SPACE_LG: f32 = 20.0;
pub const SPACE_XL: f32 = 28.0;

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

pub fn truncate_middle(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max || max < 5 {
        return s.to_string();
    }
    let keep = max - 1;
    let head = keep.div_ceil(2);
    let tail = keep - head;
    let head_s: String = chars[..head].iter().collect();
    let tail_s: String = chars[chars.len() - tail..].iter().collect();
    format!("{head_s}…{tail_s}")
}
impl NothingTheme {
    pub const ICON_SIZE: f32 = 16.0;
    pub const BLACK: Color32 = Color32::from_rgb(0, 0, 0);
    pub const SURFACE: Color32 = Color32::from_rgb(17, 17, 17);
    pub const SURFACE_RAISED: Color32 = Color32::from_rgb(26, 26, 26);
    pub const BORDER_VISIBLE: Color32 = Color32::from_rgb(51, 51, 51);
    pub const BORDER: Color32 = Color32::from_rgb(34, 34, 34);
    pub const TEXT_DISPLAY: Color32 = Color32::from_rgb(255, 255, 255);
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(232, 232, 232);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(153, 153, 153);
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(102, 102, 102);
    pub const ACCENT: Color32 = Color32::from_rgb(215, 25, 33);

    pub fn setup_fonts(ctx: &egui::Context) {
        let mut fonts = FontDefinitions::default();

        fonts.font_data.insert(
            "space_grotesk".into(),
            FontData::from_static(include_bytes!("../assets/fonts/SpaceGrotesk-Regular.ttf")),
        );
        fonts.font_data.insert(
            "space_mono".into(),
            FontData::from_static(include_bytes!("../assets/fonts/SpaceMono-Regular.ttf")),
        );

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "space_grotesk".into());

        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, "space_mono".into());

        ctx.set_fonts(fonts);
    }

    pub fn visuals() -> Visuals {
        let mut v = Visuals::dark();
        v.override_text_color = Some(Self::TEXT_PRIMARY);
        v.window_fill = Self::BLACK;
        v.panel_fill = Self::BLACK;
        v.extreme_bg_color = Self::BLACK;
        v.faint_bg_color = Self::SURFACE;
        v.widgets.noninteractive.bg_fill = Self::SURFACE;
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::TEXT_SECONDARY);
        v.widgets.inactive.bg_fill = Self::SURFACE;
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, Self::TEXT_SECONDARY);
        v.widgets.hovered.bg_fill = Self::SURFACE;
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, Self::TEXT_PRIMARY);
        v.widgets.active.bg_fill = Self::TEXT_DISPLAY;
        v.widgets.active.fg_stroke = Stroke::new(1.0, Self::BLACK);
        v.selection.bg_fill = Self::SURFACE;
        v.selection.stroke = Stroke::new(1.0, Self::TEXT_DISPLAY);
        v.window_rounding = Rounding::ZERO;
        v.window_shadow = eframe::epaint::Shadow::NONE;
        v.popup_shadow = eframe::epaint::Shadow::NONE;
        v
    }

    pub fn font_label() -> FontId {
        FontId::new(11.0, FontFamily::Monospace)
    }

    pub fn font_body() -> FontId {
        FontId::new(14.0, FontFamily::Proportional)
    }

    pub fn font_display() -> FontId {
        FontId::new(36.0, FontFamily::Monospace)
    }

    pub fn font_hero() -> FontId {
        FontId::new(48.0, FontFamily::Monospace)
    }

    pub fn font_button() -> FontId {
        FontId::new(13.0, FontFamily::Monospace)
    }
}

pub fn label_caps(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .font(NothingTheme::font_label())
            .color(NothingTheme::TEXT_SECONDARY)
            .strong(),
    );
}

pub fn setting_row<R>(
    ui: &mut egui::Ui,
    icons: &crate::icons::Icons,
    icon: Icon,
    label: &str,
    add_controls: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.allocate_ui_with_layout(
            Vec2::new(LABEL_COL, 44.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                icons.show(
                    ui,
                    icon,
                    NothingTheme::ICON_SIZE,
                    NothingTheme::TEXT_SECONDARY,
                );
                ui.add_space(6.0);
                label_caps(ui, label);
            },
        );
        add_controls(ui)
    })
    .inner
}

pub fn setting_hint(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(LABEL_COL + 14.0);
        ui.label(
            egui::RichText::new(text)
                .font(NothingTheme::font_body())
                .color(NothingTheme::TEXT_DISABLED)
                .size(11.5),
        );
    });
}

pub fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    current: &mut T,
    options: &[(T, &str)],
    enabled: bool,
    width: f32,
) {
    // Equal-width segments + a sliding pill (short ease) so selection feels
    // physical without eating interaction latency.
    // Unique id_salt per call site — shared "seg"/pill ids collide across rows.
    ui.push_id(id_salt, |ui| {
        let n = options.len().max(1) as f32;
        let btn_w = ((width - 4.0) / n).max(40.0);
        let selected_idx = options
            .iter()
            .position(|(v, _)| *v == *current)
            .unwrap_or(0) as f32;
        let anim_id = ui.id().with("pill");
        let anim_idx = ui
            .ctx()
            .animate_value_with_time(anim_id, selected_idx, 0.12);

        ui.add_enabled_ui(enabled, |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let frame = egui::Frame::none()
                .stroke(Stroke::new(1.0, NothingTheme::BORDER_VISIBLE))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(2.0));

            frame.show(ui, |ui| {
                let inner_h = 40.0;
                let total_w = btn_w * n;
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(total_w, inner_h), egui::Sense::hover());

                let pill = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + anim_idx * btn_w, rect.top()),
                    Vec2::new(btn_w, inner_h),
                );
                ui.painter()
                    .rect_filled(pill, Rounding::same(6.0), NothingTheme::TEXT_DISPLAY);

                for (i, (value, label)) in options.iter().enumerate() {
                    let seg = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + i as f32 * btn_w, rect.top()),
                        Vec2::new(btn_w, inner_h),
                    );
                    let selected = *current == *value;
                    // Cross-fade label while the pill slides under neighboring segments.
                    let prox = 1.0 - (anim_idx - i as f32).abs().clamp(0.0, 1.0);
                    let text_color = if !enabled {
                        NothingTheme::TEXT_DISABLED
                    } else {
                        lerp_color(NothingTheme::TEXT_SECONDARY, NothingTheme::BLACK, prox)
                    };

                    let resp = ui.interact(
                        seg,
                        ui.id().with(i),
                        if enabled {
                            egui::Sense::click()
                        } else {
                            egui::Sense::hover()
                        },
                    );
                    if resp.clicked() {
                        *current = *value;
                    }
                    if enabled && resp.hovered() && !selected {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    ui.painter().text(
                        seg.center(),
                        egui::Align2::CENTER_CENTER,
                        *label,
                        NothingTheme::font_button(),
                        text_color,
                    );
                }
            });
        });
    });
}

/// Single-line picker matching segmented chrome (replaces stock ComboBox).
/// One option → static selected bar. Several → click opens a matching menu.
pub fn choice_bar(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    current: &mut usize,
    labels: &[String],
    enabled: bool,
    width: f32,
) {
    if labels.is_empty() {
        return;
    }
    *current = (*current).min(labels.len() - 1);
    let multi = labels.len() > 1;
    let popup_id = ui.make_persistent_id(("choice_bar", &id_salt));

    ui.push_id(id_salt, |ui| {
        ui.add_enabled_ui(enabled, |ui| {
            let frame = egui::Frame::none()
                .stroke(Stroke::new(1.0, NothingTheme::BORDER_VISIBLE))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(2.0));

            frame.show(ui, |ui| {
                let size = Vec2::new((width - 4.0).max(40.0), 40.0);
                let sense = if enabled && multi {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                };
                let (rect, resp) = ui.allocate_exact_size(size, sense);

                ui.painter()
                    .rect_filled(rect, Rounding::same(6.0), NothingTheme::TEXT_DISPLAY);

                let label = truncate_middle(&labels[*current], 42);
                let text_galley = ui.painter().layout_no_wrap(
                    label,
                    NothingTheme::font_button(),
                    NothingTheme::BLACK,
                );
                let text_pos = if multi {
                    egui::pos2(rect.left() + 14.0, rect.center().y - text_galley.size().y / 2.0)
                } else {
                    egui::pos2(
                        rect.center().x - text_galley.size().x / 2.0,
                        rect.center().y - text_galley.size().y / 2.0,
                    )
                };
                ui.painter()
                    .galley(text_pos, text_galley, NothingTheme::BLACK);

                if multi {
                    ui.painter().text(
                        rect.right_center() - Vec2::new(14.0, 0.0),
                        egui::Align2::CENTER_CENTER,
                        "▾",
                        NothingTheme::font_label(),
                        NothingTheme::BLACK,
                    );
                }

                if enabled && multi && resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(popup_id));
                }

                egui::popup_below_widget(
                    ui,
                    popup_id,
                    &resp,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.set_min_width(rect.width());
                        egui::Frame::none()
                            .fill(NothingTheme::SURFACE_RAISED)
                            .stroke(Stroke::new(1.0, NothingTheme::BORDER_VISIBLE))
                            .inner_margin(Margin::symmetric(8.0, 6.0))
                            .show(ui, |ui| {
                                for (i, label) in labels.iter().enumerate() {
                                    let selected = i == *current;
                                    let color = if selected {
                                        NothingTheme::TEXT_DISPLAY
                                    } else {
                                        NothingTheme::TEXT_SECONDARY
                                    };
                                    let r = ui.add_sized(
                                        Vec2::new(rect.width() - 16.0, 32.0),
                                        egui::Button::new(
                                            egui::RichText::new(label)
                                                .font(NothingTheme::font_button())
                                                .color(color),
                                        )
                                        .fill(if selected {
                                            NothingTheme::SURFACE
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .stroke(Stroke::NONE)
                                        .rounding(Rounding::same(4.0)),
                                    );
                                    if r.clicked() {
                                        *current = i;
                                        ui.memory_mut(|m| m.close_popup());
                                    }
                                }
                            });
                    },
                );
            });
        });
    });
}

/// Progressive-disclosure toggle for secondary settings (Hick's Law).
pub fn more_toggle(ui: &mut egui::Ui, open: &mut bool, width: f32) -> egui::Response {
    let label = if *open { "LESS  ▲" } else { "MORE  ▼" };
    let size = Vec2::new(width, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hover_t = ui
        .ctx()
        .animate_value_with_time(resp.id.with("more_hov"), if resp.hovered() { 1.0 } else { 0.0 }, 0.1);
    let color = lerp_color(
        NothingTheme::TEXT_SECONDARY,
        NothingTheme::TEXT_PRIMARY,
        hover_t,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        NothingTheme::font_label(),
        color,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if resp.clicked() {
        *open = !*open;
    }
    resp
}

pub enum RunButtonState {
    Ready,
    Disabled,
    Processing,
}

pub enum SuggestButtonState {
    Ready,
    Disabled,
    Analyzing,
}

/// Secondary control (MORE weight) for suggest-settings.
pub fn suggest_button(
    ui: &mut egui::Ui,
    icons: &crate::icons::Icons,
    state: SuggestButtonState,
    width: f32,
) -> egui::Response {
    let enabled = matches!(state, SuggestButtonState::Ready);
    let (label, color, icon) = match state {
        SuggestButtonState::Ready => (
            "SUGGEST",
            NothingTheme::TEXT_SECONDARY,
            Icon::MagicWand,
        ),
        SuggestButtonState::Disabled => (
            "SUGGEST",
            NothingTheme::TEXT_DISABLED,
            Icon::MagicWand,
        ),
        SuggestButtonState::Analyzing => (
            "ANALYZING…",
            NothingTheme::TEXT_PRIMARY,
            Icon::CircleNotch,
        ),
    };

    let size = Vec2::new(width, 32.0);
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);

    let hover_t = ui.ctx().animate_value_with_time(
        resp.id.with("suggest_hov"),
        if enabled && resp.hovered() { 1.0 } else { 0.0 },
        0.1,
    );
    let color = if enabled {
        lerp_color(color, NothingTheme::TEXT_PRIMARY, hover_t)
    } else {
        color
    };
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let icon_d = 14.0;
    let gap = 8.0;
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), NothingTheme::font_label(), color);
    let group_w = icon_d + gap + galley.size().x;
    let start_x = rect.center().x - group_w / 2.0;
    let cy = rect.center().y;
    let spin = if matches!(state, SuggestButtonState::Analyzing) {
        ui.ctx().input(|i| i.time as f32) * 4.0
    } else {
        0.0
    };
    icons.paint_at_rotated(
        ui,
        icon,
        egui::pos2(start_x + icon_d / 2.0, cy),
        icon_d,
        color,
        spin,
    );
    ui.painter().galley(
        egui::pos2(start_x + icon_d + gap, cy - galley.size().y / 2.0),
        galley,
        color,
    );

    resp
}

pub fn run_button(
    ui: &mut egui::Ui,
    icons: &crate::icons::Icons,
    state: RunButtonState,
    width: f32,
) -> egui::Response {
    let enabled = matches!(state, RunButtonState::Ready);
    let (label, fill, text_color, stroke, icon, icon_tint) = match state {
        RunButtonState::Ready => (
            "RUN",
            NothingTheme::TEXT_DISPLAY,
            NothingTheme::BLACK,
            Stroke::NONE,
            Icon::Play,
            NothingTheme::BLACK,
        ),
        RunButtonState::Disabled => (
            "RUN",
            Color32::TRANSPARENT,
            NothingTheme::TEXT_DISABLED,
            Stroke::new(1.0, NothingTheme::BORDER),
            Icon::Play,
            NothingTheme::TEXT_DISABLED,
        ),
        RunButtonState::Processing => (
            "PROCESSING",
            Color32::TRANSPARENT,
            NothingTheme::TEXT_PRIMARY,
            Stroke::new(1.0, NothingTheme::BORDER_VISIBLE),
            Icon::CircleNotch,
            NothingTheme::TEXT_PRIMARY,
        ),
    };

    let size = Vec2::new(width, 48.0);
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(size, sense);

    // Subtle hover feedback (color shift), only when actionable.
    let hover_target = if enabled && resp.hovered() { 1.0 } else { 0.0 };
    let t = ui
        .ctx()
        .animate_value_with_time(resp.id, hover_target, 0.12);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let fill = if enabled {
        lerp_color(fill, NothingTheme::TEXT_PRIMARY, t)
    } else {
        fill
    };

    if fill != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 999.0, fill);
    }
    if stroke != Stroke::NONE {
        ui.painter().rect_stroke(rect, 999.0, stroke);
    }

    // Center the icon + label as one optical group regardless of button width.
    let icon_d = 14.0;
    let gap = 10.0;
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), NothingTheme::font_button(), text_color);
    let label_w = galley.size().x;
    let group_w = icon_d + gap + label_w;
    let start_x = rect.center().x - group_w / 2.0;
    let cy = rect.center().y;
    icons.paint_at_rotated(
        ui,
        icon,
        egui::pos2(start_x + icon_d / 2.0, cy),
        icon_d,
        icon_tint,
        0.0,
    );
    ui.painter().galley(
        egui::pos2(start_x + icon_d + gap, cy - galley.size().y / 2.0),
        galley,
        text_color,
    );

    resp
}

/// Determinate 0–1 progress bar.
pub fn progress_bar(ui: &mut egui::Ui, progress: f32, width: f32) {
    let progress = progress.clamp(0.0, 1.0);
    let h = 4.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, h), egui::Sense::hover());
    ui.painter().rect_filled(rect, 999.0, NothingTheme::BORDER);
    if progress > 0.0 {
        let fill = egui::Rect::from_min_max(
            rect.left_top(),
            egui::pos2(rect.left() + rect.width() * progress, rect.bottom()),
        );
        ui.painter()
            .rect_filled(fill, 999.0, NothingTheme::TEXT_DISPLAY);
    }
}
