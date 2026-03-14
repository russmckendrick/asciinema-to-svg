use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashMap;
use std::path::Path;

const BUILT_IN_SIZES: &str = include_str!("../config/sizes.json");

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
    pub size: String,

    /// Path to a custom sizes JSON file (overrides built-in presets)
    #[arg(long)]
    pub size_config: Option<String>,

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

/// Resolve the scale factor for the given size preset name.
///
/// If `config_path` is provided, presets are loaded from that file instead of
/// the built-in `config/sizes.json`.
pub fn resolve_scale_factor(size: &str, config_path: Option<&str>) -> Result<f32> {
    let map: HashMap<String, f32> = match config_path {
        Some(path) => {
            let contents = std::fs::read_to_string(Path::new(path))
                .with_context(|| format!("failed to read sizes config {}", path))?;
            serde_json::from_str(&contents)
                .with_context(|| format!("failed to parse sizes config {}", path))?
        }
        None => {
            serde_json::from_str(BUILT_IN_SIZES).context("failed to parse built-in sizes.json")?
        }
    };

    map.get(size).copied().with_context(|| {
        let mut available: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
        available.sort();
        format!(
            "unknown size '{}'. Available presets: {}",
            size,
            available.join(", ")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_builds() {
        Cli::command().debug_assert();
    }

    #[test]
    fn resolves_built_in_presets() {
        let small = resolve_scale_factor("small", None).unwrap();
        let medium = resolve_scale_factor("medium", None).unwrap();
        let large = resolve_scale_factor("large", None).unwrap();
        assert!((small - 0.61).abs() < 0.001);
        assert!((medium - 0.78).abs() < 0.001);
        assert!((large - 1.0).abs() < 0.001);
    }

    #[test]
    fn rejects_unknown_preset() {
        assert!(resolve_scale_factor("tiny", None).is_err());
    }

    #[test]
    fn resolves_from_custom_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom.json");
        std::fs::write(&path, r#"{"compact": 0.5, "wide": 1.2}"#).unwrap();
        let factor = resolve_scale_factor("compact", Some(path.to_str().unwrap())).unwrap();
        assert!((factor - 0.5).abs() < 0.001);
    }
}
