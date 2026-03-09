use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    pub fn scale_factor(self) -> f32 {
        match self {
            Size::Small => 0.61,
            Size::Medium => 0.78,
            Size::Large => 1.0,
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "asciinema-to-svg",
    version,
    about = "Convert asciinema v2/v3 cast files into themed animated SVG terminal recordings"
)]
pub struct Cli {
    /// Input asciicast file path
    pub input: String,

    /// Output SVG path
    #[arg(short, long, default_value = "output.svg")]
    pub output: String,

    /// Built-in theme name or a path to a theme JSON file
    #[arg(long)]
    pub theme: Option<String>,

    /// Output size preset (small, medium, large)
    #[arg(long, default_value = "medium")]
    pub size: Size,

    /// Explicit output width in pixels
    #[arg(long)]
    pub width: Option<u32>,

    /// Explicit output height in pixels
    #[arg(long)]
    pub height: Option<u32>,

    /// Override the terminal window title
    #[arg(long)]
    pub title: Option<String>,

    /// Disable statusline prompt remapping
    #[arg(long)]
    pub no_statusline: bool,

    /// Path to a standalone statusline config JSON (overrides theme prompt section)
    #[arg(long)]
    pub statusline: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_builds() {
        Cli::command().debug_assert();
    }
}
