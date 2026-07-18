mod app;
mod drop;
mod icons;
mod models;
mod onnx;
mod paths;
mod preview;
mod theme;
mod worker;

use app::UpscaleApp;
use eframe::egui;

/// Build the window/taskbar icon: the white Loku magic-wand glyph composited on
/// a dark rounded square, so it reads as a real app mark rather than a bare icon.
fn load_app_icon() -> egui::IconData {
    let glyph = image::load_from_memory(include_bytes!("../assets/icon/loku-glyph.png"))
        .expect("loku glyph png")
        .to_rgba8();

    let size = 256u32;
    let radius = 52.0f32;
    let bg = [13u8, 13, 13, 255];
    let mut img = image::RgbaImage::from_pixel(size, size, image::Rgba(bg));

    // Rounded corners: drop alpha outside the corner radius.
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            let dx = if fx < radius {
                radius - fx
            } else if fx > size as f32 - radius {
                fx - (size as f32 - radius)
            } else {
                0.0
            };
            let dy = if fy < radius {
                radius - fy
            } else if fy > size as f32 - radius {
                fy - (size as f32 - radius)
            } else {
                0.0
            };
            if dx > 0.0 && dy > 0.0 && (dx * dx + dy * dy).sqrt() > radius {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }
    }

    // Alpha-composite the centered glyph over the rounded background.
    let (gw, gh) = (glyph.width(), glyph.height());
    let ox = (size - gw) / 2;
    let oy = (size - gh) / 2;
    for y in 0..gh {
        for x in 0..gw {
            let p = glyph.get_pixel(x, y).0;
            let a = p[3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let b = img.get_pixel(ox + x, oy + y).0;
            let mix = |f: u8, g: u8| (f as f32 * a + g as f32 * (1.0 - a)).round() as u8;
            let out_a = (p[3] as f32 + b[3] as f32 * (1.0 - a)).round() as u8;
            img.put_pixel(
                ox + x,
                oy + y,
                image::Rgba([mix(p[0], b[0]), mix(p[1], b[1]), mix(p[2], b[2]), out_a]),
            );
        }
    }

    egui::IconData {
        rgba: img.into_raw(),
        width: size,
        height: size,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 800.0])
            .with_min_inner_size([460.0, 620.0])
            .with_icon(load_app_icon())
            .with_title("Loku"),
        ..Default::default()
    };

    eframe::run_native(
        "Loku",
        options,
        Box::new(|cc| Ok(Box::new(UpscaleApp::new(cc)))),
    )
}
