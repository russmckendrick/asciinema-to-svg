mod cast;
mod cli;
mod render;
mod terminal;
mod theme;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use std::path::Path;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let theme = theme::ThemeDefinition::load(cli.theme.as_deref())?;
    let session = cast::RecordingSession::read_from_file(Path::new(&cli.input))?;
    let title = cli
        .title
        .clone()
        .or_else(|| {
            Path::new(&cli.input)
                .file_stem()
                .map(|value| value.to_string_lossy().to_string())
        })
        .or_else(|| Some("Terminal".to_string()));

    let statusline_config = cli
        .statusline
        .as_deref()
        .map(theme::PromptTheme::load_from_file)
        .transpose()?;

    let svg = render::render_animated_svg(
        &session,
        &theme,
        render::RenderOptions {
            width_px: cli.width,
            height_px: cli.height,
            window_title: title,
            statusline: !cli.no_statusline,
            statusline_config,
        },
    )?;

    std::fs::write(&cli.output, svg)
        .with_context(|| format!("failed to write SVG output to {}", cli.output))?;

    Ok(())
}
