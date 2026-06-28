#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Model {
    #[default]
    AnimeV3,
    X4Plus,
    X4PlusAnime,
}

impl Model {
    pub fn cli_name(self) -> &'static str {
        match self {
            Model::AnimeV3 => "realesr-animevideov3",
            Model::X4Plus => "realesrgan-x4plus",
            Model::X4PlusAnime => "realesrgan-x4plus-anime",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Png,
    Jpg,
    Webp,
}

impl OutputFormat {
    pub fn ext(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Jpg => "jpg",
            OutputFormat::Webp => "webp",
        }
    }
}
