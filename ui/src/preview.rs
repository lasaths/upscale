use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MAX_EDGE: u32 = 720;

pub struct PreviewCache {
    textures: HashMap<PathBuf, TextureHandle>,
}

impl PreviewCache {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub fn texture(&mut self, ctx: &egui::Context, path: &Path) -> Option<&TextureHandle> {
        if path.is_file() && !self.textures.contains_key(path) {
            if let Some(handle) = load_texture(ctx, path) {
                self.textures.insert(path.to_path_buf(), handle);
            }
        }
        self.textures.get(path)
    }

    pub fn reload(&mut self, ctx: &egui::Context, path: &Path) {
        self.textures.remove(path);
        self.texture(ctx, path);
    }
}

fn load_texture(ctx: &egui::Context, path: &Path) -> Option<TextureHandle> {
    let img = image::open(path).ok()?;
    let img = img.thumbnail(MAX_EDGE, MAX_EDGE);
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba);
    Some(ctx.load_texture(
        format!("preview:{}", path.display()),
        color_image,
        TextureOptions::LINEAR,
    ))
}
