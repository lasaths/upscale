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

pub fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    options: &[(T, &str)],
    enabled: bool,
    width: f32,
) {
    // Equal-width buttons that exactly fill `width`, so every segmented control
    // shares one right edge and the settings strip reads as a clean grid.
    let n = options.len().max(1) as f32;
    let btn_w = ((width - 4.0) / n).max(40.0);
    ui.add_enabled_ui(enabled, |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let frame = egui::Frame::none()
            .stroke(Stroke::new(1.0, NothingTheme::BORDER_VISIBLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(Margin::same(2.0));

        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (value, label) in options {
                    let selected = *current == *value;
                    let fill = if selected {
                        NothingTheme::TEXT_DISPLAY
                    } else {
                        Color32::TRANSPARENT
                    };
                    let text_color = if selected {
                        NothingTheme::BLACK
                    } else if enabled {
                        NothingTheme::TEXT_SECONDARY
                    } else {
                        NothingTheme::TEXT_DISABLED
                    };

                    let btn = egui::Button::new(
                        egui::RichText::new(*label)
                            .font(NothingTheme::font_button())
                            .color(text_color),
                    )
                    .fill(fill)
                    .stroke(Stroke::NONE)
                    .rounding(Rounding::same(6.0))
                    .min_size(Vec2::new(btn_w, 40.0));

                    if ui.add_sized(Vec2::new(btn_w, 40.0), btn).clicked() {
                        *current = *value;
                    }
                }
            });
        });
    });
}

pub enum RunButtonState {
    Ready,
    Disabled,
    Processing,
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
