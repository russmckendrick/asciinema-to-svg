mod cast;
mod cli;
mod icons;
mod render;
mod terminal;
mod theme;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use std::path::Path;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut theme = theme::ThemeDefinition::load(cli.theme.as_deref())?;
    let scale_factor = cli::resolve_scale_factor(&cli.size, cli.size_config.as_deref())?;
    theme.scale(scale_factor);
    let session = cast::RecordingSession::read_from_file(Path::new(&cli.input))?;

    // The cast file stem is only a *fallback* — render will prefer an explicit
    // --title, then any OSC 0/2 title captured during replay, before this.
    let fallback_title = Path::new(&cli.input)
        .file_stem()
        .map(|value| value.to_string_lossy().to_string());

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
            window_title: cli.title.clone(),
            fallback_title,
            statusline: !cli.no_statusline,
            statusline_config,
            speed: cli.speed,
            idle_time_limit: cli.idle_time_limit,
            start: cli.start,
            end: cli.end,
            at: cli.at,
            loop_animation: !cli.no_loop,
        },
    )?;

    std::fs::write(&cli.output, svg)
        .with_context(|| format!("failed to write SVG output to {}", cli.output))?;

    Ok(())
}
