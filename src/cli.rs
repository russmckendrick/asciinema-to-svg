use clap::Parser;

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

    /// Explicit output width in pixels
    #[arg(long)]
    pub width: Option<u32>,

    /// Explicit output height in pixels
    #[arg(long)]
    pub height: Option<u32>,

    /// Override the terminal window title
    #[arg(long)]
    pub title: Option<String>,

    /// Disable powerline/starship prompt remapping
    #[arg(long)]
    pub no_powerline: bool,
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
