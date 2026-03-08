use crate::cast::RecordingSession;
use crate::terminal::{TerminalEmulator, TerminalFrame, screen_buffer::ScreenCell};
use crate::theme::{ChromeKind, ThemeDefinition};
use anyhow::Result;
use std::fmt::Write;

pub struct RenderOptions {
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
    pub window_title: Option<String>,
    pub powerline: bool,
}

struct Layout {
    width: f32,
    height: f32,
    frame_x: f32,
    frame_y: f32,
    terminal_width: f32,
    terminal_height: f32,
    cell_width: f32,
    line_height: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptSegment {
    text: String,
    fill: String,
}

pub fn render_animated_svg(
    session: &RecordingSession,
    theme: &ThemeDefinition,
    options: RenderOptions,
) -> Result<String> {
    let mut emulator = TerminalEmulator::new(
        session.terminal_size.width,
        session.terminal_size.height,
        theme,
    );
    let mut frames = emulator.replay(session);
    normalize_frame_timing(&mut frames);
    let (content_cols, content_rows) = measure_content_bounds(&frames, theme);

    let natural_width = theme.chrome.padding * 2.0 + content_cols as f32 * theme.font_size * 0.62;
    let natural_height = theme.chrome.padding * 2.0
        + theme.chrome.title_bar_height
        + content_rows as f32 * theme.line_height;

    let mut width = options
        .width_px
        .map(|value| value as f32)
        .unwrap_or(natural_width);
    let mut height = options
        .height_px
        .map(|value| value as f32)
        .unwrap_or(natural_height);

    if options.width_px.is_some() && options.height_px.is_none() {
        height = width * natural_height / natural_width;
    } else if options.width_px.is_none() && options.height_px.is_some() {
        width = height * natural_width / natural_height;
    }

    let layout = Layout {
        width,
        height,
        frame_x: theme.chrome.padding,
        frame_y: theme.chrome.padding + theme.chrome.title_bar_height,
        terminal_width: (width - theme.chrome.padding * 2.0).max(1.0),
        terminal_height: (height - theme.chrome.padding * 2.0 - theme.chrome.title_bar_height)
            .max(1.0),
        cell_width: ((width - theme.chrome.padding * 2.0)
            / session.terminal_size.width.max(1) as f32)
            .max(theme.font_size * 0.45),
        line_height: ((height - theme.chrome.padding * 2.0 - theme.chrome.title_bar_height)
            / session.terminal_size.height.max(1) as f32)
            .max(theme.line_height),
    };

    let title = options
        .window_title
        .unwrap_or_else(|| "Terminal".to_string());

    let total_duration = frames
        .last()
        .map(|frame| frame.time.max(0.2) + 0.2)
        .unwrap_or(0.2);
    let mut svg = String::new();

    writeln!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" viewBox="0 0 {:.0} {:.0}" role="img" aria-label="Animated terminal recording" data-theme="{}">"#,
        layout.width,
        layout.height,
        layout.width,
        layout.height,
        escape_xml(&theme.name)
    )?;
    svg.push_str("<defs>");
    append_styles(&mut svg, theme, total_duration)?;
    svg.push_str("</defs>");
    append_window_chrome(&mut svg, theme, &layout, &title)?;
    svg.push_str(&format!(
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
        layout.frame_x,
        layout.frame_y,
        layout.terminal_width,
        layout.terminal_height,
        theme.terminal.background
    ));

    for (index, frame) in frames.iter().enumerate() {
        append_frame(
            &mut svg,
            theme,
            &layout,
            frame,
            index,
            total_duration,
            options.powerline,
        )?;
    }

    svg.push_str("</svg>");
    Ok(svg)
}

fn append_styles(svg: &mut String, theme: &ThemeDefinition, duration: f64) -> Result<()> {
    writeln!(
        svg,
        r#"<style>
        .terminal-text {{
            font-family: {};
            font-size: {}px;
            dominant-baseline: hanging;
            white-space: pre;
        }}
        .prompt-text {{
            font-family: {};
            font-size: {}px;
            dominant-baseline: middle;
        }}
        .frame {{
            opacity: 0;
            animation-duration: {}s;
            animation-timing-function: steps(1, end);
            animation-iteration-count: infinite;
        }}
        </style>"#,
        css_text(&theme.font_family),
        theme.font_size,
        css_text(&theme.prompt.font_family),
        theme.prompt.font_size,
        duration
    )?;
    Ok(())
}

fn append_window_chrome(
    svg: &mut String,
    theme: &ThemeDefinition,
    layout: &Layout,
    title: &str,
) -> Result<()> {
    writeln!(
        svg,
        r#"<rect x="0" y="0" width="{:.2}" height="{:.2}" rx="{:.2}" fill="{}" stroke="{}"/>"#,
        layout.width,
        layout.height,
        theme.chrome.radius,
        theme.chrome.background,
        theme.chrome.border_color
    )?;
    writeln!(
        svg,
        r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="14" fill="{}" dominant-baseline="middle"{}>{}</text>"#,
        match theme.chrome.kind {
            ChromeKind::Macos | ChromeKind::Linux => layout.width / 2.0,
            ChromeKind::Powershell => 12.0,
        },
        theme.chrome.padding + theme.chrome.title_bar_height / 2.0,
        css_text("ui-sans-serif, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"),
        theme.chrome.title_color,
        if matches!(theme.chrome.kind, ChromeKind::Macos | ChromeKind::Linux) {
            r#" text-anchor="middle""#
        } else {
            ""
        },
        escape_xml(title)
    )?;

    match theme.chrome.kind {
        ChromeKind::Macos => {
            for (index, color) in ["#ff5f57", "#febc2e", "#28c840"].iter().enumerate() {
                writeln!(
                    svg,
                    r#"<circle cx="{:.2}" cy="{:.2}" r="7" fill="{}"/>"#,
                    theme.chrome.padding + 18.0 + 22.0 * index as f32,
                    theme.chrome.padding + theme.chrome.title_bar_height / 2.0,
                    color
                )?;
            }
            writeln!(
                svg,
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.5" opacity="0.55"/>"#,
                theme.chrome.padding + 100.0,
                theme.chrome.padding + theme.chrome.title_bar_height / 2.0,
                layout.width - (theme.chrome.padding + 100.0),
                theme.chrome.padding + theme.chrome.title_bar_height / 2.0,
                theme.chrome.subtitle_color
            )?;
        }
        ChromeKind::Linux => {
            writeln!(
                svg,
                r##"<circle cx="18" cy="{:.2}" r="8" fill="#dd4814"/><circle cx="42" cy="{:.2}" r="8" fill="#666666"/><circle cx="66" cy="{:.2}" r="8" fill="#888888"/>"##,
                theme.chrome.padding + theme.chrome.title_bar_height / 2.0,
                theme.chrome.padding + theme.chrome.title_bar_height / 2.0,
                theme.chrome.padding + theme.chrome.title_bar_height / 2.0
            )?;
        }
        ChromeKind::Powershell => {
            let y = theme.chrome.padding + theme.chrome.title_bar_height / 2.0;
            writeln!(
                svg,
                r#"<text x="{:.2}" y="{:.2}" font-family="Segoe UI, sans-serif" font-size="12" fill="{}" dominant-baseline="middle">_</text>"#,
                layout.width - 70.0,
                y,
                theme.chrome.subtitle_color
            )?;
            writeln!(
                svg,
                r#"<rect x="{:.2}" y="{:.2}" width="10" height="10" fill="none" stroke="{}"/>"#,
                layout.width - 46.0,
                y - 5.0,
                theme.chrome.subtitle_color
            )?;
            writeln!(
                svg,
                r#"<text x="{:.2}" y="{:.2}" font-family="Segoe UI, sans-serif" font-size="12" fill="{}" dominant-baseline="middle">×</text>"#,
                layout.width - 18.0,
                y,
                theme.chrome.subtitle_color
            )?;
        }
    }

    Ok(())
}

fn append_frame(
    svg: &mut String,
    theme: &ThemeDefinition,
    layout: &Layout,
    frame: &TerminalFrame,
    index: usize,
    total_duration: f64,
    powerline: bool,
) -> Result<()> {
    let start = if total_duration <= 0.0 {
        0.0
    } else {
        (frame.time / total_duration * 100.0).clamp(0.0, 100.0)
    };
    let end = 100.0;
    writeln!(
        svg,
        r#"<g class="frame" style="animation-name: frame-{};">"#,
        index
    )?;
    writeln!(
        svg,
        r#"<style>@keyframes frame-{} {{ 0%, {:.3}% {{ opacity: 0; }} {:.3}%, {}% {{ opacity: 1; }} }}</style>"#,
        index, start, start, end
    )?;

    for row_index in 0..frame.buffer.height {
        let row = frame.buffer.row(row_index);
        if powerline {
            if let Some(prompt) = detect_powerline_prompt(row, theme) {
                append_prompt(svg, theme, layout, row_index, &prompt)?;
                continue;
            }
        }
        append_row_text(svg, layout, theme, row_index, row)?;
    }
    svg.push_str("</g>");
    Ok(())
}

fn append_row_text(
    svg: &mut String,
    layout: &Layout,
    theme: &ThemeDefinition,
    row_index: usize,
    row: &[ScreenCell],
) -> Result<()> {
    let y = layout.frame_y + row_index as f32 * layout.line_height + 4.0;
    for (column, cell) in row.iter().enumerate() {
        if cell.is_wide_continuation || cell.text == " " {
            continue;
        }
        let x = layout.frame_x + column as f32 * layout.cell_width + 4.0;
        let background = effective_background(cell);
        if background != theme.terminal.background {
            writeln!(
                svg,
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}" opacity="0.95"/>"#,
                layout.frame_x + column as f32 * layout.cell_width,
                layout.frame_y + row_index as f32 * layout.line_height,
                if cell.is_wide {
                    layout.cell_width * 2.0
                } else {
                    layout.cell_width
                },
                layout.line_height,
                background
            )?;
        }
        writeln!(
            svg,
            r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}"{}{}>{}</text>"#,
            x,
            y,
            effective_foreground(cell),
            if cell.bold {
                r#" font-weight="700""#
            } else {
                ""
            },
            if cell.italic {
                r#" font-style="italic""#
            } else {
                ""
            },
            escape_xml(&cell.text)
        )?;
        if cell.underline {
            writeln!(
                svg,
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.2"/>"#,
                x,
                y + theme.font_size + 1.0,
                x + layout.cell_width * if cell.is_wide { 2.0 } else { 1.0 },
                y + theme.font_size + 1.0,
                effective_foreground(cell)
            )?;
        }
    }
    Ok(())
}

fn append_prompt(
    svg: &mut String,
    theme: &ThemeDefinition,
    layout: &Layout,
    row_index: usize,
    prompt: &[PromptSegment],
) -> Result<()> {
    let mut cursor_x = layout.frame_x + 4.0;
    let y = layout.frame_y + row_index as f32 * layout.line_height + 2.0;
    let height = theme
        .prompt
        .segment_height
        .min(layout.line_height - 4.0)
        .max(18.0);
    let top = y;
    let center_y = top + height / 2.0;

    for (index, segment) in prompt.iter().enumerate() {
        let is_blank = segment.text.is_empty();
        let width = if is_blank {
            (theme.prompt.segment_height * 0.88).max(26.0)
        } else {
            theme.prompt.row_padding_x * 2.0
                + segment.text.chars().count() as f32 * theme.prompt.font_size * 0.62
                + 22.0
        };
        let color = &segment.fill;
        let next_color = prompt
            .get(index + 1)
            .map(|value| value.fill.as_str())
            .unwrap_or(theme.prompt.separator_fill.as_str());

        if index == 0 {
            writeln!(
                svg,
                r#"<path d="M {:.2} {:.2} h {:.2} l 16 {:.2} l -16 {:.2} h -18 a 12 12 0 0 1 -12 -12 a 12 12 0 0 1 12 -12 z" fill="{}"/>"#,
                cursor_x + 12.0,
                top,
                width - 12.0,
                height / 2.0,
                height / 2.0,
                color
            )?;
            writeln!(
                svg,
                r#"<text class="prompt-text" x="{:.2}" y="{:.2}" fill="{}">{}</text>"#,
                cursor_x + 20.0,
                center_y,
                theme.prompt.text_color,
                escape_xml(&theme.prompt.leading_symbol)
            )?;
        } else {
            writeln!(
                svg,
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}" opacity="0.95"/>"#,
                cursor_x,
                top,
                cursor_x + 18.0,
                center_y,
                cursor_x,
                top + height,
                theme.prompt.edge_fill
            )?;
            writeln!(
                svg,
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                cursor_x + 10.0,
                top,
                width - 10.0,
                height,
                color
            )?;
        }

        if !is_blank {
            writeln!(
                svg,
                r#"<text class="prompt-text" x="{:.2}" y="{:.2}" fill="{}">{}</text>"#,
                cursor_x + theme.prompt.row_padding_x + 28.0,
                center_y,
                theme.prompt.text_color,
                escape_xml(&segment.text)
            )?;
        }

        let right_x = cursor_x + width;
        writeln!(
            svg,
            r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}"/>"#,
            right_x - 6.0,
            top,
            right_x + 18.0,
            center_y,
            right_x - 6.0,
            top + height,
            next_color
        )?;
        cursor_x = right_x + 8.0;
    }

    writeln!(
        svg,
        r##"<text class="prompt-text" x="{:.2}" y="{:.2}" fill="#39ff14">{}</text>"##,
        cursor_x + 6.0,
        center_y,
        escape_xml(&theme.prompt.trailing_symbol)
    )?;

    Ok(())
}

fn detect_powerline_prompt(
    row: &[ScreenCell],
    theme: &ThemeDefinition,
) -> Option<Vec<PromptSegment>> {
    if !row.iter().any(|cell| {
        !cell.is_wide_continuation
            && (cell.text.contains('')
                || cell.text.contains('')
                || cell.text.contains('')
                || cell.text.contains('')
                || cell.text.contains(''))
    }) {
        return None;
    }

    let mut segments = Vec::new();
    let mut current_fill: Option<&str> = None;
    let mut current_text = String::new();

    for cell in row {
        if cell.is_wide_continuation {
            continue;
        }

        let background = effective_background(cell);
        if background.eq_ignore_ascii_case(&theme.terminal.background) {
            if let Some(fill) = current_fill.take() {
                segments.push(PromptSegment {
                    text: normalize_prompt_segment(&current_text),
                    fill: fill.to_string(),
                });
                current_text.clear();
            }
            continue;
        }

        match current_fill {
            Some(fill) if fill.eq_ignore_ascii_case(background) => {
                current_text.push_str(&cell.text);
            }
            Some(fill) => {
                segments.push(PromptSegment {
                    text: normalize_prompt_segment(&current_text),
                    fill: fill.to_string(),
                });
                current_text.clear();
                current_text.push_str(&cell.text);
                current_fill = Some(background);
            }
            None => {
                current_text.push_str(&cell.text);
                current_fill = Some(background);
            }
        }
    }

    if let Some(fill) = current_fill {
        segments.push(PromptSegment {
            text: normalize_prompt_segment(&current_text),
            fill: fill.to_string(),
        });
    }

    let cleaned = segments
        .into_iter()
        .filter(|segment| {
            !segment
                .fill
                .eq_ignore_ascii_case(&theme.terminal.background)
        })
        .collect::<Vec<_>>();

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn normalize_prompt_segment(segment: &str) -> String {
    segment
        .chars()
        .filter(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    ' ' | '~'
                        | '.'
                        | '/'
                        | '_'
                        | '-'
                        | '+'
                        | ':'
                        | '['
                        | ']'
                        | '('
                        | ')'
                        | '{'
                        | '}'
                        | '!'
                        | '?'
                        | '@'
                        | '\\'
                )
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn effective_foreground(cell: &ScreenCell) -> &str {
    if cell.reversed {
        &cell.background
    } else {
        &cell.foreground
    }
}

fn effective_background(cell: &ScreenCell) -> &str {
    if cell.reversed {
        &cell.foreground
    } else {
        &cell.background
    }
}

fn normalize_frame_timing(frames: &mut [TerminalFrame]) {
    if frames.is_empty() {
        return;
    }
    let mut last_time = 0.0f64;
    for (index, frame) in frames.iter_mut().enumerate() {
        if index == 0 {
            frame.time = frame.time.max(0.05);
        } else {
            frame.time = frame.time.max(last_time + 0.05);
        }
        last_time = frame.time;
    }
}

fn measure_content_bounds(frames: &[TerminalFrame], theme: &ThemeDefinition) -> (usize, usize) {
    let mut max_row = 0usize;
    let mut max_col = 0usize;

    for frame in frames {
        for row_index in 0..frame.buffer.height {
            let row = frame.buffer.row(row_index);
            let mut row_has_content = false;
            for (col_index, cell) in row.iter().enumerate() {
                if cell.is_wide_continuation {
                    continue;
                }
                let has_content = cell.text != " "
                    || !effective_background(cell).eq_ignore_ascii_case(&theme.terminal.background);
                if has_content {
                    row_has_content = true;
                    max_col = max_col.max(col_index + if cell.is_wide { 2 } else { 1 });
                }
            }
            if row_has_content {
                max_row = max_row.max(row_index + 1);
            }
        }
    }

    let max_width = frames.first().map(|frame| frame.buffer.width).unwrap_or(48);
    let max_height = frames.first().map(|frame| frame.buffer.height).unwrap_or(6);
    let min_width = 48.min(max_width);
    let min_height = 6.min(max_height);

    (
        max_col.clamp(min_width, max_width),
        max_row.clamp(min_height, max_height),
    )
}

fn css_text(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cast::RecordingSession;

    #[test]
    fn renders_svg_with_macos_theme() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = RecordingSession::read_from_str(
            r#"{"version":2,"width":20,"height":4,"timestamp":0}
[0.1,"o","hello"]
"#,
        )
        .unwrap();
        let svg = render_animated_svg(
            &session,
            &theme,
            RenderOptions {
                width_px: None,
                height_px: None,
                window_title: Some("demo".to_string()),
                powerline: true,
            },
        )
        .unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("demo"));
    }

    #[test]
    fn detects_powerline_prompts() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = RecordingSession::read_from_str(
            r#"{"version":2,"width":40,"height":4,"timestamp":0}
[0.1,"o","\u001b[38;2;214;93;14m\u001b[48;2;214;93;14;38;2;251;241;199m󰀵 russ.mckendrick \u001b[48;2;215;153;33;38;2;214;93;14m\u001b[38;2;251;241;199m ~ \u001b[48;2;104;157;106;38;2;215;153;33m\u001b[48;2;69;133;136;38;2;104;157;106m\u001b[48;2;131;165;152;38;2;69;133;136m\u001b[0m\u001b[38;2;131;165;152m\u001b[0m\r\n"]
"#,
        )
        .unwrap();
        let mut emulator = TerminalEmulator::new(40, 4, &theme);
        let frames = emulator.replay(&session);
        let prompt = detect_powerline_prompt(frames[0].buffer.row(0), &theme).unwrap();
        assert_eq!(prompt[0].text, "russ.mckendrick");
        assert_eq!(prompt[1].text, "~");
        assert!(prompt.len() >= 4);
    }

    #[test]
    fn respects_no_powerline_flag() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = RecordingSession::read_from_str(
            r#"{"version":2,"width":40,"height":4,"timestamp":0}
[0.1,"o","russ  repo\r\n"]
"#,
        )
        .unwrap();
        let svg = render_animated_svg(
            &session,
            &theme,
            RenderOptions {
                width_px: None,
                height_px: None,
                window_title: None,
                powerline: false,
            },
        )
        .unwrap();
        assert!(!svg.contains(""));
    }
}
