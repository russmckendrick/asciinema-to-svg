use crate::terminal::screen_buffer::ScreenCell;
use crate::theme::PromptTheme;
use anyhow::Result;
use std::fmt::Write;

/// Check whether a row contains any powerline separator glyph.
pub fn is_powerline_row(row: &[ScreenCell]) -> bool {
    row.iter()
        .any(|cell| !cell.is_wide_continuation && is_powerline_separator(&cell.text))
}

/// Render a bespoke statusline using the theme's `prompt.segments` text.
///
/// Each segment gets a background color from `prompt.palette` (cycling),
/// a right-pointing arrow separator, and the segment text centered vertically.
/// Height matches `line_height` so the statusline is the same height as text rows.
pub fn render_bespoke_statusline(
    svg: &mut String,
    prompt: &PromptTheme,
    frame_x: f32,
    row_y: f32,
    terminal_width: f32,
    line_height: f32,
    terminal_bg: &str,
) -> Result<()> {
    if prompt.segments.is_empty() {
        return Ok(());
    }

    let palette_len = prompt.palette.len();
    let arrow_width: f32 = 14.0;
    let padding_x = prompt.segment_padding_x.unwrap_or(prompt.row_padding_x);
    let h = line_height;
    let text_y = row_y + h / 2.0;
    let char_w = prompt.font_size * 0.6;

    // Fill the full row with terminal background first
    writeln!(
        svg,
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
        frame_x, row_y, terminal_width, h, terminal_bg
    )?;

    let mut x = frame_x;

    for (i, segment_text) in prompt.segments.iter().enumerate() {
        let pi = i % palette_len;
        let bg = &prompt.palette[pi];
        let text_width = segment_text.len() as f32 * char_w;
        let seg_width = (padding_x * 2.0 + text_width).max(padding_x * 2.0 + 8.0);

        // Background rect
        writeln!(
            svg,
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" class="statusline-seg"/>"#,
            x, row_y, seg_width, h, bg
        )?;

        // Text label
        if !segment_text.is_empty() {
            writeln!(
                svg,
                r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{}" fill="{}" dominant-baseline="central" class="statusline-text">{}</text>"#,
                x + padding_x, text_y,
                super::css_text(&prompt.font_family),
                prompt.font_size, prompt.text_color,
                super::escape_xml(segment_text)
            )?;
        }

        x += seg_width;

        // Arrow separator
        let next_bg = if i + 1 < prompt.segments.len() {
            &prompt.palette[(pi + 1) % palette_len]
        } else {
            terminal_bg
        };

        // Background fill behind arrow
        writeln!(
            svg,
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
            x, row_y, arrow_width, h, next_bg
        )?;
        // Arrow triangle
        writeln!(
            svg,
            r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" class="statusline-arrow"/>"#,
            x, row_y, x + arrow_width, row_y + h / 2.0, x, row_y + h, bg
        )?;

        x += arrow_width;
    }

    Ok(())
}

/// All powerline separator glyphs (both solid and thin variants).
fn is_powerline_separator(text: &str) -> bool {
    let ch = match text.chars().next() {
        Some(c) if text.chars().count() == 1 => c,
        _ => return false,
    };
    matches!(
        ch,
        '\u{E0B0}' | '\u{E0B1}' | '\u{E0B2}' | '\u{E0B3}'
            | '\u{E0B4}' | '\u{E0B5}' | '\u{E0B6}' | '\u{E0B7}'
            | '\u{E0B8}' | '\u{E0B9}' | '\u{E0BA}' | '\u{E0BB}'
            | '\u{E0BC}' | '\u{E0BD}' | '\u{E0BE}' | '\u{E0BF}'
    )
}

/// Returns true for any Unicode Private Use Area codepoint.
pub fn is_private_use_area(text: &str) -> bool {
    text.chars().any(|ch| {
        let cp = ch as u32;
        (0xE000..=0xF8FF).contains(&cp)
            || (0xF0000..=0xFFFFD).contains(&cp)
            || (0x100000..=0x10FFFD).contains(&cp)
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::PromptTheme;

    fn cell(text: &str) -> ScreenCell {
        ScreenCell {
            text: text.to_string(),
            foreground: "#ffffff".to_string(),
            background: "#000000".to_string(),
            bold: false,
            italic: false,
            underline: false,
            reversed: false,
            faint: false,
            is_wide: false,
            is_wide_continuation: false,
        }
    }

    fn test_prompt() -> PromptTheme {
        PromptTheme {
            font_family: "monospace".to_string(),
            font_size: 17.0,
            row_padding_x: 12.0,
            segment_height: 28.0,
            text_color: "#f5e9c9".to_string(),
            edge_fill: "#4d9ea5".to_string(),
            separator_fill: "#7ab39d".to_string(),
            leading_symbol: "\u{E0B6}".to_string(),
            trailing_symbol: "\u{276F}".to_string(),
            palette: vec![
                "#d96d0f".to_string(),
                "#d7a126".to_string(),
                "#78a85e".to_string(),
            ],
            segment_padding_x: None,
            segments: vec!["user".to_string(), "~".to_string()],
        }
    }

    #[test]
    fn detects_powerline_row_e0b0() {
        let row = vec![cell("u"), cell("s"), cell("e"), cell("r"), cell("\u{E0B0}"), cell("~")];
        assert!(is_powerline_row(&row));
    }

    #[test]
    fn detects_powerline_row_e0ba() {
        let row = vec![cell("x"), cell("\u{E0BA}"), cell("y")];
        assert!(is_powerline_row(&row));
    }

    #[test]
    fn detects_non_powerline_row() {
        let row = vec![cell("h"), cell("e"), cell("l"), cell("l"), cell("o")];
        assert!(!is_powerline_row(&row));
    }

    #[test]
    fn render_bespoke_produces_svg_elements() {
        let prompt = test_prompt();
        let mut svg = String::new();
        render_bespoke_statusline(&mut svg, &prompt, 16.0, 62.0, 400.0, 28.0, "#232744").unwrap();
        assert!(svg.contains("user"));
        assert!(svg.contains("~"));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("statusline-seg"));
        assert!(svg.contains("statusline-arrow"));
    }

    #[test]
    fn render_bespoke_empty_segments_produces_nothing() {
        let mut prompt = test_prompt();
        prompt.segments = vec![];
        let mut svg = String::new();
        render_bespoke_statusline(&mut svg, &prompt, 16.0, 62.0, 400.0, 28.0, "#232744").unwrap();
        assert!(svg.is_empty());
    }

    #[test]
    fn render_bespoke_no_pua_chars() {
        let prompt = test_prompt();
        let mut svg = String::new();
        render_bespoke_statusline(&mut svg, &prompt, 16.0, 62.0, 400.0, 28.0, "#232744").unwrap();
        for ch in svg.chars() {
            let cp = ch as u32;
            assert!(
                !(0xE000..=0xF8FF).contains(&cp),
                "Found PUA char U+{:04X} in SVG output",
                cp
            );
        }
    }
}
