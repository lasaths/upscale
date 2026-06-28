#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Model {
    #[default]
    AnimeV3,
    X4Plus,
    X4PlusAnime,
}

impl Model {
    #[allow(dead_code)]
    pub const ALL: [Model; 3] = [Model::AnimeV3, Model::X4Plus, Model::X4PlusAnime];

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            Model::AnimeV3 => "animev3",
            Model::X4Plus => "x4plus",
            Model::X4PlusAnime => "x4plus-anime",
        }
    }

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
    #[allow(dead_code)]
    pub const ALL: [OutputFormat; 3] = [OutputFormat::Png, OutputFormat::Jpg, OutputFormat::Webp];

    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            OutputFormat::Png => "PNG",
            OutputFormat::Jpg => "JPG",
            OutputFormat::Webp => "WEBP",
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Jpg => "jpg",
            OutputFormat::Webp => "webp",
        }
    }

    pub fn cli_flag(self) -> &'static str {
        self.ext()
    }
}
