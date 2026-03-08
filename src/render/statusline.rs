use crate::terminal::screen_buffer::ScreenCell;
use crate::theme::PromptTheme;
use anyhow::Result;
use std::fmt::Write;

/// A single segment extracted from a powerline prompt region.
#[derive(Debug, Clone)]
pub struct PowerlineSegment {
    pub text: String,
    /// SVG icon paths to prepend before text (from nerd font glyphs).
    pub icons: Vec<&'static str>,
}

/// The parsed layout of a powerline row:
/// left prompt segments, a middle gap (plain text), and right prompt segments.
#[derive(Debug)]
pub struct StatuslineLayout {
    pub left: Vec<PowerlineSegment>,
    pub right: Vec<PowerlineSegment>,
    /// Column range of the middle gap (start_col, end_col) — plain text area.
    pub middle_start_col: usize,
    pub middle_end_col: usize,
}

/// Check whether a row contains any powerline separator glyph.
pub fn is_powerline_row(row: &[ScreenCell]) -> bool {
    row.iter()
        .any(|cell| !cell.is_wide_continuation && is_powerline_separator(&cell.text))
}

/// Parse a powerline row into left segments, middle gap, and right segments.
///
/// Strategy: find separator positions, then identify a large gap of default-bg
/// spaces between them. Everything left of the gap is left-prompt, everything
/// right is right-prompt. The gap itself is the middle area for plain text.
pub fn parse_row(row: &[ScreenCell], terminal_bg: &str) -> StatuslineLayout {
    // Collect separator column indices
    let sep_cols: Vec<usize> = row
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.is_wide_continuation && is_powerline_separator(&c.text))
        .map(|(i, _)| i)
        .collect();

    if sep_cols.is_empty() {
        return StatuslineLayout {
            left: vec![],
            right: vec![],
            middle_start_col: 0,
            middle_end_col: row.len(),
        };
    }

    // Find the largest gap of default-bg spaces between consecutive separators.
    // The gap marks the boundary between left-prompt and right-prompt.
    let mut best_gap_start = 0usize;
    let mut best_gap_end = 0usize;
    let mut best_gap_len = 0usize;

    for window in sep_cols.windows(2) {
        let left_sep = window[0];
        let right_sep = window[1];
        // Count contiguous default-bg spaces between these separators
        let gap_start = left_sep + 1;
        let gap_end = right_sep;
        let space_count = row[gap_start..gap_end]
            .iter()
            .filter(|c| {
                !c.is_wide_continuation
                    && c.text.trim().is_empty()
                    && effective_bg(c).eq_ignore_ascii_case(terminal_bg)
            })
            .count();
        let total_cells = row[gap_start..gap_end]
            .iter()
            .filter(|c| !c.is_wide_continuation)
            .count();

        // A gap must be mostly spaces (>50% and at least 3 cells) to qualify
        if space_count > 3 && space_count * 2 > total_cells && space_count > best_gap_len {
            best_gap_len = space_count;
            best_gap_start = gap_start;
            best_gap_end = gap_end;
        }
    }

    // Also check gap after the last separator (trailing space to end of row)
    if let Some(&last_sep) = sep_cols.last() {
        let gap_start = last_sep + 1;
        let gap_end = row.len();
        let space_count = row[gap_start..gap_end]
            .iter()
            .filter(|c| {
                !c.is_wide_continuation
                    && c.text.trim().is_empty()
                    && effective_bg(c).eq_ignore_ascii_case(terminal_bg)
            })
            .count();
        let total_cells = row[gap_start..gap_end]
            .iter()
            .filter(|c| !c.is_wide_continuation)
            .count();
        if space_count > 3 && space_count * 2 > total_cells && space_count > best_gap_len {
            best_gap_len = space_count;
            best_gap_start = gap_start;
            best_gap_end = gap_end;
        }
    }

    // Also check gap before the first separator
    if let Some(&first_sep) = sep_cols.first() {
        if first_sep > 0 {
            let gap_start = 0;
            let gap_end = first_sep;
            let space_count = row[gap_start..gap_end]
                .iter()
                .filter(|c| {
                    !c.is_wide_continuation
                        && c.text.trim().is_empty()
                        && effective_bg(c).eq_ignore_ascii_case(terminal_bg)
                })
                .count();
            let total_cells = row[gap_start..gap_end]
                .iter()
                .filter(|c| !c.is_wide_continuation)
                .count();
            if space_count > 3 && space_count * 2 > total_cells && space_count > best_gap_len {
                best_gap_len = space_count;
                best_gap_start = gap_start;
                best_gap_end = gap_end;
            }
        }
    }

    if best_gap_len == 0 {
        // No significant gap found — treat entire row as left prompt
        let segments = extract_segments_from_range(row, 0, row.len());
        return StatuslineLayout {
            left: segments,
            right: vec![],
            middle_start_col: row.len(),
            middle_end_col: row.len(),
        };
    }

    // Split: left prompt is everything before the gap, right prompt after
    let left_end = best_gap_start;
    let right_start = best_gap_end;

    let left = extract_segments_from_range(row, 0, left_end);
    let right = extract_segments_from_range(row, right_start, row.len());

    StatuslineLayout {
        left,
        right,
        middle_start_col: best_gap_start,
        middle_end_col: best_gap_end,
    }
}

/// Extract powerline segments from a sub-range of cells.
fn extract_segments_from_range(
    row: &[ScreenCell],
    start: usize,
    end: usize,
) -> Vec<PowerlineSegment> {
    let mut segments: Vec<PowerlineSegment> = Vec::new();
    let mut current_text = String::new();
    let mut current_icons: Vec<&'static str> = Vec::new();

    for cell in &row[start..end] {
        if cell.is_wide_continuation {
            continue;
        }
        if is_powerline_separator(&cell.text) {
            let trimmed = current_text.trim().to_string();
            if !trimmed.is_empty() || !current_icons.is_empty() {
                segments.push(PowerlineSegment {
                    text: trimmed,
                    icons: current_icons,
                });
            }
            current_text.clear();
            current_icons = Vec::new();
        } else if let Some(icon_path) = nerd_font_svg_path(&cell.text) {
            current_icons.push(icon_path);
        } else if is_private_use_area(&cell.text) {
            // Unknown PUA glyph — skip
        } else {
            current_text.push_str(&cell.text);
        }
    }

    let trimmed = current_text.trim().to_string();
    if !trimmed.is_empty() || !current_icons.is_empty() {
        segments.push(PowerlineSegment {
            text: trimmed,
            icons: current_icons,
        });
    }

    segments
}

/// Render left-aligned statusline segments. Returns the x position after the last arrow.
pub fn render_segments_left(
    svg: &mut String,
    segments: &[PowerlineSegment],
    prompt: &PromptTheme,
    start_x: f32,
    row_y: f32,
    terminal_bg: &str,
    palette_offset: usize,
) -> Result<f32> {
    if segments.is_empty() {
        return Ok(start_x);
    }

    let palette_len = prompt.palette.len();
    let arrow_width: f32 = 14.0;
    let icon_size: f32 = 14.0;
    let icon_gap: f32 = 6.0;
    let padding_x = prompt.segment_padding_x.unwrap_or(prompt.row_padding_x);
    let h = prompt.segment_height;
    let text_y = row_y + h / 2.0;
    let char_w = prompt.font_size * 0.6;

    let mut x = start_x;

    for (i, segment) in segments.iter().enumerate() {
        let pi = (palette_offset + i) % palette_len;
        let bg = &prompt.palette[pi];
        let icon_width = segment.icons.len() as f32 * (icon_size + icon_gap);
        let text_width = segment.text.len() as f32 * char_w;
        let seg_width = (padding_x * 2.0 + icon_width + text_width).max(padding_x * 2.0 + 8.0);

        // Background rect
        writeln!(
            svg,
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" class="statusline-seg"/>"#,
            x, row_y, seg_width, h, bg
        )?;

        // Icons
        let mut content_x = x + padding_x;
        for icon_path in &segment.icons {
            let icon_y = row_y + (h - icon_size) / 2.0;
            writeln!(
                svg,
                r#"<g transform="translate({:.2},{:.2}) scale({:.4})"><path d="{}" fill="{}"/></g>"#,
                content_x, icon_y, icon_size / 16.0, icon_path, prompt.text_color
            )?;
            content_x += icon_size + icon_gap;
        }

        // Text label
        if !segment.text.is_empty() {
            writeln!(
                svg,
                r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{}" fill="{}" dominant-baseline="central" class="statusline-text">{}</text>"#,
                content_x, text_y,
                super::css_text(&prompt.font_family),
                prompt.font_size, prompt.text_color,
                super::escape_xml(&segment.text)
            )?;
        }

        x += seg_width;

        // Arrow separator
        let next_bg = if i + 1 < segments.len() {
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

    Ok(x)
}

/// Render right-aligned statusline segments. Returns the starting x.
pub fn render_segments_right(
    svg: &mut String,
    segments: &[PowerlineSegment],
    prompt: &PromptTheme,
    right_edge: f32,
    row_y: f32,
    terminal_bg: &str,
    palette_offset: usize,
) -> Result<f32> {
    if segments.is_empty() {
        return Ok(right_edge);
    }

    let palette_len = prompt.palette.len();
    let arrow_width: f32 = 14.0;
    let icon_size: f32 = 14.0;
    let icon_gap: f32 = 6.0;
    let padding_x = prompt.segment_padding_x.unwrap_or(prompt.row_padding_x);
    let h = prompt.segment_height;
    let text_y = row_y + h / 2.0;
    let char_w = prompt.font_size * 0.6;

    // Calculate total width of right segments
    let mut total_width: f32 = 0.0;
    for segment in segments {
        let icon_width = segment.icons.len() as f32 * (icon_size + icon_gap);
        let text_width = segment.text.len() as f32 * char_w;
        let seg_width = (padding_x * 2.0 + icon_width + text_width).max(padding_x * 2.0 + 8.0);
        total_width += seg_width + arrow_width;
    }

    let start_x = right_edge - total_width;
    let mut x = start_x;

    for (i, segment) in segments.iter().enumerate() {
        let pi = (palette_offset + i) % palette_len;
        let bg = &prompt.palette[pi];
        let icon_width = segment.icons.len() as f32 * (icon_size + icon_gap);
        let text_width = segment.text.len() as f32 * char_w;
        let seg_width = (padding_x * 2.0 + icon_width + text_width).max(padding_x * 2.0 + 8.0);

        // Left-pointing arrow (entrance from terminal bg into segment)
        let prev_bg = if i == 0 { terminal_bg } else { &prompt.palette[(palette_offset + i - 1) % palette_len] };
        // Background behind arrow
        writeln!(
            svg,
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
            x, row_y, arrow_width, h, prev_bg
        )?;
        // Left-pointing arrow triangle
        writeln!(
            svg,
            r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" class="statusline-arrow"/>"#,
            x + arrow_width, row_y, x, row_y + h / 2.0, x + arrow_width, row_y + h, bg
        )?;
        x += arrow_width;

        // Background rect
        writeln!(
            svg,
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" class="statusline-seg"/>"#,
            x, row_y, seg_width, h, bg
        )?;

        // Icons
        let mut content_x = x + padding_x;
        for icon_path in &segment.icons {
            let icon_y = row_y + (h - icon_size) / 2.0;
            writeln!(
                svg,
                r#"<g transform="translate({:.2},{:.2}) scale({:.4})"><path d="{}" fill="{}"/></g>"#,
                content_x, icon_y, icon_size / 16.0, icon_path, prompt.text_color
            )?;
            content_x += icon_size + icon_gap;
        }

        // Text label
        if !segment.text.is_empty() {
            writeln!(
                svg,
                r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{}" fill="{}" dominant-baseline="central" class="statusline-text">{}</text>"#,
                content_x, text_y,
                super::css_text(&prompt.font_family),
                prompt.font_size, prompt.text_color,
                super::escape_xml(&segment.text)
            )?;
        }

        x += seg_width;
    }

    Ok(start_x)
}

fn effective_bg(cell: &ScreenCell) -> &str {
    if cell.reversed {
        &cell.foreground
    } else {
        &cell.background
    }
}

// ---------------------------------------------------------------------------
// Glyph classification
// ---------------------------------------------------------------------------

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

/// Map well-known nerd font codepoints to SVG path data (16x16 viewBox).
fn nerd_font_svg_path(text: &str) -> Option<&'static str> {
    let ch = text.chars().next()?;
    if text.chars().count() != 1 {
        return None;
    }
    match ch {
        '\u{F015}' => Some("M8 1L1 7v8h5v-4h4v4h5V7L8 1z"),
        '\u{F07C}' => Some("M1 3v10h14l-2-7H6V3H1zm5 1v2h7l1.5 5H2V4h4z"),
        '\u{F126}' => Some("M12 2a2 2 0 0 0-2 2c0 .7.4 1.4 1 1.7V7H9L5 9.3V5.7A2 2 0 0 0 4 2a2 2 0 0 0-2 2c0 .9.6 1.6 1.3 1.9v4.2A2 2 0 0 0 4 14a2 2 0 0 0 1.7-3l4-2.3h1.6c.2.9 1 1.6 2 1.6a2.3 2.3 0 0 0 0-4.6c-1 0-1.8.7-2 1.6H9.6L13 5.7c.6-.3 1-1 1-1.7a2 2 0 0 0-2-2z"),
        '\u{F113}' => Some("M8 0a8 8 0 0 0-2.5 15.6c.4 0 .5-.2.5-.4v-1.5C3.8 14.1 3.3 12.6 3.3 12.6c-.4-.9-.9-1.2-.9-1.2-.7-.5 0-.5 0-.5.8 0 1.2.8 1.2.8.7 1.2 1.9.9 2.3.7.1-.5.3-.9.5-1-1.8-.2-3.7-.9-3.7-4 0-.9.3-1.6.8-2.2-.1-.2-.4-1 .1-2.1 0 0 .7-.2 2.2.8a7.4 7.4 0 0 1 4 0c1.5-1 2.2-.8 2.2-.8.5 1.1.2 1.9.1 2.1.5.6.8 1.3.8 2.2 0 3.1-1.9 3.8-3.7 4 .3.3.6.8.6 1.6v2.4c0 .2.1.5.6.4A8 8 0 0 0 8 0z"),
        '\u{F179}' => Some("M12.2 8.4c0-2-1.6-3-1.7-3a3.2 3.2 0 0 1 2.7-1.5C11.6 1.3 10 2.8 10 2.8S9 2 7.8 2C6 2 4 3.8 4 6.4 4 10 6 14 7.6 14c.8 0 1.6-.8 2.4-.8.8 0 1.4.8 2.2.8C14 14 15 10 15 10s-2.8-.5-2.8-1.6zM10 1.5C10.5.7 11.3 0 11.3 0s-.1 1-.7 1.8c-.5.7-1.2 1.2-1.2 1.2s0-1 .6-1.5z"),
        '\u{F252}' => Some("M3 1v2h1v2a4 4 0 0 0 1.5 3L7 9.5 5.5 11A4 4 0 0 0 4 14v1H3v1h10v-1h-1v-1a4 4 0 0 0-1.5-3L9 9.5l1.5-1.5A4 4 0 0 0 12 5V3h1V1H3zm3 4V3h4v2a2 2 0 0 1-.8 1.6L8 7.8 6.8 6.6A2 2 0 0 1 6 5zm2 5.2 1.2 1.2c.5.5.8 1 .8 1.6v2H6v-2c0-.6.3-1.2.8-1.6L8 10.2z"),
        '\u{F00C}' | '\u{2714}' => Some("M2 8l4 4 8-8-1.5-1.5L6 9 3.5 6.5z"),
        '\u{E73C}' => Some("M4.5 1L0 8l4.5 7h7L16 8 11.5 1h-7zm1 2h5L14 8l-3.5 5h-5L2 8l3.5-5z"),
        '\u{EB70}' => Some("M1 2v12h14V2H1zm13 11H2V5h12v8zM5.5 7L3 9.5 5.5 12l1-1L4.5 9.5 6.5 8l-1-1zM8 11h4v1H8v-1z"),
        '\u{F0035}' => Some("M6 1v4H2v2h4v2H2v2h4v4h2v-4h2v4h2v-4h2V9h-2V7h2V5h-2V1h-2v4H8V1H6zm2 6h2v2H8V7z"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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

    fn cell_bg(text: &str, bg: &str) -> ScreenCell {
        ScreenCell {
            text: text.to_string(),
            foreground: "#ffffff".to_string(),
            background: bg.to_string(),
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
            segment_height: 42.0,
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
    fn parse_row_splits_left_right() {
        // Simulate: [apple] [sep] ~ [sep] (gap of spaces) [sep] ✔ [sep] base
        let term_bg = "#000000";
        let mut row: Vec<ScreenCell> = Vec::new();

        // Left prompt: icon, separator, text, separator
        row.push(cell_bg("\u{F179}", "#333")); // apple icon
        row.push(cell_bg(" ", "#333"));
        row.push(cell_bg("\u{E0BC}", "#333")); // separator
        row.push(cell_bg(" ", "#444"));
        row.push(cell_bg("~", "#444"));
        row.push(cell_bg(" ", "#444"));
        row.push(cell_bg("\u{E0BC}", "#444")); // separator — end of left prompt

        // Gap (10 default-bg spaces)
        for _ in 0..10 {
            row.push(cell_bg(" ", term_bg));
        }

        // Right prompt: separator, text, separator, text
        row.push(cell_bg("\u{E0BA}", term_bg)); // separator
        row.push(cell_bg(" ", "#555"));
        row.push(cell_bg("✔", "#555"));
        row.push(cell_bg(" ", "#555"));
        row.push(cell_bg("\u{E0BA}", "#555")); // separator
        row.push(cell_bg(" ", "#666"));
        row.push(cell_bg("b", "#666"));
        row.push(cell_bg("a", "#666"));
        row.push(cell_bg("s", "#666"));
        row.push(cell_bg("e", "#666"));
        row.push(cell_bg(" ", "#666"));

        let layout = parse_row(&row, term_bg);
        assert!(!layout.left.is_empty(), "should have left segments: {:?}", layout.left);
        assert!(!layout.right.is_empty(), "should have right segments: {:?}", layout.right);
        // Left should contain "~"
        assert!(layout.left.iter().any(|s| s.text == "~"), "left segments: {:?}", layout.left);
        // Right should contain "base"
        assert!(layout.right.iter().any(|s| s.text.contains("base")), "right segments: {:?}", layout.right);
    }

    #[test]
    fn unknown_pua_chars_are_dropped() {
        let row = vec![cell("\u{E100}"), cell("a"), cell("\u{E0B0}"), cell("b")];
        let segments = extract_segments_from_range(&row, 0, row.len());
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "a");
        assert!(segments[0].icons.is_empty());
        assert_eq!(segments[1].text, "b");
    }

    #[test]
    fn render_left_produces_svg_elements() {
        let prompt = test_prompt();
        let segments = vec![
            PowerlineSegment { text: "user".into(), icons: vec![] },
            PowerlineSegment { text: "~/code".into(), icons: vec![] },
        ];
        let mut svg = String::new();
        let end_x = render_segments_left(&mut svg, &segments, &prompt, 16.0, 62.0, "#232744", 0).unwrap();
        assert!(end_x > 16.0);
        assert!(svg.contains("user"));
        assert!(svg.contains("~/code"));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<polygon"));
    }

    #[test]
    fn render_no_nerd_font_chars_in_output() {
        let prompt = test_prompt();
        let segments = vec![
            PowerlineSegment {
                text: "user".into(),
                icons: vec![nerd_font_svg_path("\u{F179}").unwrap()],
            },
        ];
        let mut svg = String::new();
        render_segments_left(&mut svg, &segments, &prompt, 16.0, 62.0, "#232744", 0).unwrap();
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
