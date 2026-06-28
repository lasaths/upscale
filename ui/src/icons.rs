use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions, Ui, Vec2};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum Icon {
    UploadSimple,
    Cpu,
    ArrowsOut,
    FilePng,
    Queue,
    CircleNotch,
    Play,
    Sparkle,
    Check,
    Warning,
    Image,
    MagicWand,
}

pub struct Icons {
    textures: HashMap<Icon, TextureHandle>,
}

impl Icons {
    pub fn new(ctx: &egui::Context) -> Self {
        let mut textures = HashMap::new();
        textures.insert(
            Icon::UploadSimple,
            load_png(
                ctx,
                "upload-simple",
                include_bytes!("../assets/icons/upload-simple-light.png"),
            ),
        );
        textures.insert(
            Icon::Cpu,
            load_png(ctx, "cpu", include_bytes!("../assets/icons/cpu-light.png")),
        );
        textures.insert(
            Icon::ArrowsOut,
            load_png(
                ctx,
                "arrows-out",
                include_bytes!("../assets/icons/arrows-out-light.png"),
            ),
        );
        textures.insert(
            Icon::FilePng,
            load_png(
                ctx,
                "file-png",
                include_bytes!("../assets/icons/file-png-light.png"),
            ),
        );
        textures.insert(
            Icon::Queue,
            load_png(
                ctx,
                "queue",
                include_bytes!("../assets/icons/queue-light.png"),
            ),
        );
        textures.insert(
            Icon::CircleNotch,
            load_png(
                ctx,
                "circle-notch",
                include_bytes!("../assets/icons/circle-notch-light.png"),
            ),
        );
        textures.insert(
            Icon::Play,
            load_png(
                ctx,
                "play",
                include_bytes!("../assets/icons/play-light.png"),
            ),
        );
        textures.insert(
            Icon::Sparkle,
            load_png(
                ctx,
                "sparkle",
                include_bytes!("../assets/icons/sparkle-light.png"),
            ),
        );
        textures.insert(
            Icon::Check,
            load_png(
                ctx,
                "check",
                include_bytes!("../assets/icons/check-light.png"),
            ),
        );
        textures.insert(
            Icon::Warning,
            load_png(
                ctx,
                "warning",
                include_bytes!("../assets/icons/warning-light.png"),
            ),
        );
        textures.insert(
            Icon::Image,
            load_png(
                ctx,
                "image",
                include_bytes!("../assets/icons/image-light.png"),
            ),
        );
        textures.insert(
            Icon::MagicWand,
            load_png(
                ctx,
                "magic-wand",
                include_bytes!("../assets/icons/magic-wand-light.png"),
            ),
        );
        Self { textures }
    }

    pub fn show(&self, ui: &mut Ui, icon: Icon, size: f32, tint: Color32) {
        self.show_rotated(ui, icon, size, tint, 0.0);
    }

    pub fn show_rotated(&self, ui: &mut Ui, icon: Icon, size: f32, tint: Color32, angle: f32) {
        let Some(tex) = self.textures.get(&icon) else {
            return;
        };
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
        let mut image = egui::Image::new((tex.id(), Vec2::splat(size)))
            .fit_to_exact_size(Vec2::splat(size))
            .tint(tint);
        if angle != 0.0 {
            image = image.rotate(angle, rect.center().to_vec2());
        }
        ui.put(rect, image);
    }

    pub fn paint_at(&self, ui: &mut Ui, icon: Icon, center: egui::Pos2, size: f32, tint: Color32) {
        self.paint_at_rotated(ui, icon, center, size, tint, 0.0);
    }

    pub fn paint_at_rotated(
        &self,
        ui: &mut Ui,
        icon: Icon,
        center: egui::Pos2,
        size: f32,
        tint: Color32,
        angle: f32,
    ) {
        if angle == 0.0 {
            let Some(tex) = self.textures.get(&icon) else {
                return;
            };
            let rect = egui::Rect::from_center_size(center, Vec2::splat(size));
            ui.painter().image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                tint,
            );
        } else {
            let rect = egui::Rect::from_center_size(center, Vec2::splat(size));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
                self.show_rotated(ui, icon, size, tint, angle);
            });
        }
    }
}

fn load_png(ctx: &egui::Context, name: &str, bytes: &[u8]) -> TextureHandle {
    let img = image::load_from_memory(bytes).expect("icon png").to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let image = ColorImage::from_rgba_unmultiplied(size, &img);
    ctx.load_texture(format!("icon:{name}"), image, TextureOptions::LINEAR)
}
