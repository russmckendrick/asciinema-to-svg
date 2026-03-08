use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

const MACOS_THEME: &str = include_str!("../themes/macos.json");
const LINUX_THEME: &str = include_str!("../themes/linux.json");
const POWERSHELL_THEME: &str = include_str!("../themes/powershell.json");

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeDefinition {
    pub name: String,
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub terminal: TerminalTheme,
    pub chrome: ChromeTheme,
    pub prompt: PromptTheme,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TerminalTheme {
    pub background: String,
    pub foreground: String,
    #[allow(dead_code)]
    pub selection: String,
    pub ansi_palette: [String; 16],
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChromeTheme {
    pub kind: ChromeKind,
    pub background: String,
    pub border_color: String,
    pub title_color: String,
    pub subtitle_color: String,
    pub radius: f32,
    pub padding: f32,
    pub title_bar_height: f32,
    #[serde(default = "default_content_top_gap")]
    pub content_top_gap: f32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChromeKind {
    Macos,
    Linux,
    Powershell,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Segment {
    Text(String),
    Rich {
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        icon: Option<String>,
    },
}

impl Segment {
    pub fn text(&self) -> Option<&str> {
        match self {
            Segment::Text(s) => Some(s.as_str()),
            Segment::Rich { text, .. } => text.as_deref(),
        }
    }

    pub fn icon(&self) -> Option<&str> {
        match self {
            Segment::Text(_) => None,
            Segment::Rich { icon, .. } => icon.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct PromptTheme {
    pub font_family: String,
    pub font_size: f32,
    pub row_padding_x: f32,
    pub segment_height: f32,
    pub text_color: String,
    pub edge_fill: String,
    pub separator_fill: String,
    pub leading_symbol: String,
    pub trailing_symbol: String,
    pub palette: Vec<String>,
    #[serde(default)]
    pub segment_padding_x: Option<f32>,
    #[serde(default)]
    pub segments: Vec<Segment>,
}

impl PromptTheme {
    pub fn load_from_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(Path::new(path))
            .with_context(|| format!("failed to read statusline config file {}", path))?;
        let theme: Self = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse statusline config file {}", path))?;
        if theme.palette.is_empty() {
            anyhow::bail!("statusline palette must not be empty");
        }
        Ok(theme)
    }
}

impl ThemeDefinition {
    pub fn load(value: Option<&str>) -> Result<Self> {
        match value.unwrap_or("macos") {
            "macos" => Self::from_json(MACOS_THEME).context("failed to load built-in macos theme"),
            "linux" => Self::from_json(LINUX_THEME).context("failed to load built-in linux theme"),
            "powershell" => Self::from_json(POWERSHELL_THEME)
                .context("failed to load built-in powershell theme"),
            path => {
                let contents = std::fs::read_to_string(Path::new(path))
                    .with_context(|| format!("failed to read theme file {}", path))?;
                Self::from_json(&contents)
                    .with_context(|| format!("failed to parse theme file {}", path))
            }
        }
    }

    fn from_json(contents: &str) -> Result<Self> {
        let theme: Self = serde_json::from_str(contents)?;
        if theme.prompt.palette.is_empty() {
            anyhow::bail!("theme prompt palette must not be empty");
        }
        Ok(theme)
    }

    pub fn ansi_color(&self, index: usize) -> &str {
        &self.terminal.ansi_palette[index.min(15)]
    }

    pub fn ansi256_color(&self, index: u8) -> String {
        if index < 16 {
            return self.ansi_color(index as usize).to_string();
        }
        if index >= 232 {
            let gray = (8 + (index as i32 - 232) * 10).clamp(0, 255) as u8;
            return format!("#{:02X}{:02X}{:02X}", gray, gray, gray);
        }

        let cube = (index - 16) as i32;
        let r = cube / 36;
        let g = (cube % 36) / 6;
        let b = cube % 6;

        let rr = if r == 0 { 0 } else { 55 + r * 40 } as u8;
        let gg = if g == 0 { 0 } else { 55 + g * 40 } as u8;
        let bb = if b == 0 { 0 } else { 55 + b * 40 } as u8;
        format!("#{:02X}{:02X}{:02X}", rr, gg, bb)
    }
}

fn default_content_top_gap() -> f32 {
    8.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_built_in_theme() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        assert_eq!(theme.name, "macos");
        assert_eq!(theme.chrome.kind, ChromeKind::Macos);
    }

    #[test]
    fn rejects_invalid_custom_theme() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.json");
        std::fs::write(&path, r#"{"name":"broken","prompt":{"palette":[]}}"#).unwrap();

        let result = ThemeDefinition::load(Some(path.to_string_lossy().as_ref()));
        assert!(result.is_err());
    }

    #[test]
    fn segment_text_string_deserializes() {
        let seg: Segment = serde_json::from_str(r#""user""#).unwrap();
        assert_eq!(seg.text(), Some("user"));
        assert_eq!(seg.icon(), None);
    }

    #[test]
    fn segment_rich_object_deserializes() {
        let seg: Segment =
            serde_json::from_str(r#"{"text": "user", "icon": "apple-fill"}"#).unwrap();
        assert_eq!(seg.text(), Some("user"));
        assert_eq!(seg.icon(), Some("apple-fill"));
    }

    #[test]
    fn segment_icon_only_deserializes() {
        let seg: Segment = serde_json::from_str(r#"{"icon": "folder-fill"}"#).unwrap();
        assert_eq!(seg.text(), None);
        assert_eq!(seg.icon(), Some("folder-fill"));
    }

    #[test]
    fn mixed_segments_array_deserializes() {
        let segs: Vec<Segment> =
            serde_json::from_str(r#"["plain", {"text": "dir", "icon": "folder-fill"}]"#).unwrap();
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].text(), Some("plain"));
        assert_eq!(segs[0].icon(), None);
        assert_eq!(segs[1].text(), Some("dir"));
        assert_eq!(segs[1].icon(), Some("folder-fill"));
    }
}
