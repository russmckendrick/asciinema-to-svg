mod statusline;

use crate::cast::RecordingSession;
use crate::terminal::{TerminalEmulator, TerminalFrame, screen_buffer::ScreenCell};
use crate::theme::{ChromeKind, PromptTheme, ThemeDefinition};
use anyhow::Result;
use std::fmt::Write;

pub struct RenderOptions {
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
    pub window_title: Option<String>,
    pub statusline: bool,
    pub statusline_config: Option<PromptTheme>,
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

    let natural_cell_width = theme.font_size * 0.6;
    let natural_line_height = theme.line_height;
    let content_top_gap = theme.chrome.content_top_gap;
    let natural_terminal_width = session.terminal_size.width as f32 * natural_cell_width;

    // Each statusline row with command text occupies an extra line_height
    // (one for the statusline bar, one for the "$ command" line below it).
    // Find the maximum extra lines needed across all frames.
    let extra_statusline_rows = if options.statusline {
        frames
            .iter()
            .map(|frame| {
                let mut extra = 0usize;
                let mut first = true;
                for row_idx in 0..frame.buffer.height {
                    let row = frame.buffer.row(row_idx);
                    if statusline::is_statusline_row(row) {
                        if first {
                            // First statusline row gets an extra line for itself
                            extra += 1;
                            first = false;
                        }
                    }
                }
                extra
            })
            .max()
            .unwrap_or(0)
    } else {
        0
    };
    let natural_terminal_height =
        (session.terminal_size.height as f32 + extra_statusline_rows as f32) * natural_line_height;
    let natural_width = theme.chrome.padding * 2.0 + natural_terminal_width;
    let natural_height = theme.chrome.padding * 2.0
        + theme.chrome.title_bar_height
        + content_top_gap
        + natural_terminal_height;

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

    let layout = if options.width_px.is_none() && options.height_px.is_none() {
        Layout {
            width,
            height,
            frame_x: theme.chrome.padding,
            frame_y: theme.chrome.padding + theme.chrome.title_bar_height + content_top_gap,
            terminal_width: natural_terminal_width,
            terminal_height: natural_terminal_height,
            cell_width: natural_cell_width,
            line_height: natural_line_height,
        }
    } else {
        Layout {
            width,
            height,
            frame_x: theme.chrome.padding,
            frame_y: theme.chrome.padding + theme.chrome.title_bar_height + content_top_gap,
            terminal_width: (width - theme.chrome.padding * 2.0).max(1.0),
            terminal_height: (height
                - theme.chrome.padding * 2.0
                - theme.chrome.title_bar_height
                - content_top_gap)
                .max(1.0),
            cell_width: ((width - theme.chrome.padding * 2.0)
                / session.terminal_size.width.max(1) as f32)
                .max(theme.font_size * 0.52),
            line_height: ((height
                - theme.chrome.padding * 2.0
                - theme.chrome.title_bar_height
                - content_top_gap)
                / session.terminal_size.height.max(1) as f32)
                .max(theme.line_height),
        }
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
    writeln!(
        svg,
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
        layout.frame_x,
        layout.frame_y,
        layout.terminal_width,
        layout.terminal_height,
        theme.terminal.background
    )?;

    for (index, frame) in frames.iter().enumerate() {
        let next_frame_time = frames
            .get(index + 1)
            .map(|next| next.time)
            .unwrap_or(total_duration);
        append_frame(
            &mut svg,
            theme,
            &layout,
            frame,
            next_frame_time,
            index,
            total_duration,
            options.statusline,
            options.statusline_config.as_ref(),
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
            font-weight: 400;
            dominant-baseline: hanging;
            white-space: pre;
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
    let title_bar_top = 0.0;
    let title_bar_bottom = theme.chrome.padding + theme.chrome.title_bar_height;
    let title_bar_center_y = title_bar_top + (title_bar_bottom - title_bar_top) / 2.0;

    writeln!(
        svg,
        r#"<rect x="0" y="0" width="{:.2}" height="{:.2}" rx="{:.2}" fill="{}" stroke="{}"/>"#,
        layout.width,
        layout.height,
        theme.chrome.radius,
        theme.chrome.background,
        theme.chrome.border_color
    )?;

    if matches!(theme.chrome.kind, ChromeKind::Macos) {
        let radius = (theme.chrome.radius - 2.0).max(0.0);
        writeln!(
            svg,
            r##"<path d="M 1.00 {:.2} A {:.2} {:.2} 0 0 1 {:.2} 1.00 L {:.2} 1.00 A {:.2} {:.2} 0 0 1 {:.2} {:.2} L {:.2} {:.2} L 1.00 {:.2} Z" fill="#2a3157" opacity="0.82"/>"##,
            radius + 1.0,
            radius,
            radius,
            radius + 1.0,
            layout.width - radius - 1.0,
            radius,
            radius,
            layout.width - 1.0,
            radius + 1.0,
            layout.width - 1.0,
            title_bar_bottom,
            title_bar_bottom
        )?;
        writeln!(
            svg,
            r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#3a4677" stroke-width="1" opacity="0.8"/>"##,
            0.0,
            title_bar_bottom + 0.5,
            layout.width,
            title_bar_bottom + 0.5
        )?;
    }

    let title_font_size = if matches!(theme.chrome.kind, ChromeKind::Macos) {
        theme.chrome.title_bar_height * 0.425
    } else {
        theme.chrome.title_bar_height * 0.35
    };
    writeln!(
        svg,
        r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" dominant-baseline="middle"{}>{}</text>"#,
        match theme.chrome.kind {
            ChromeKind::Macos | ChromeKind::Linux => layout.width / 2.0,
            ChromeKind::Powershell => 12.0,
        },
        title_bar_center_y + 0.5,
        css_text("ui-sans-serif, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"),
        title_font_size,
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
                    title_bar_center_y + 0.5,
                    color
                )?;
            }
        }
        ChromeKind::Linux => {
            let y = theme.chrome.padding + theme.chrome.title_bar_height / 2.0;
            writeln!(
                svg,
                r##"<circle cx="18" cy="{:.2}" r="8" fill="#dd4814"/><circle cx="42" cy="{:.2}" r="8" fill="#666666"/><circle cx="66" cy="{:.2}" r="8" fill="#888888"/>"##,
                y, y, y
            )?;
        }
        ChromeKind::Powershell => {
            let y = theme.chrome.padding + theme.chrome.title_bar_height / 2.0;
            let ctrl_font = theme.chrome.title_bar_height * 0.3;
            writeln!(
                svg,
                r#"<text x="{:.2}" y="{:.2}" font-family="Segoe UI, sans-serif" font-size="{:.1}" fill="{}" dominant-baseline="middle">_</text>"#,
                layout.width - 70.0,
                y,
                ctrl_font,
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
                r#"<text x="{:.2}" y="{:.2}" font-family="Segoe UI, sans-serif" font-size="{:.1}" fill="{}" dominant-baseline="middle">×</text>"#,
                layout.width - 18.0,
                y,
                ctrl_font,
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
    next_frame_time: f64,
    index: usize,
    total_duration: f64,
    statusline: bool,
    statusline_config: Option<&PromptTheme>,
) -> Result<()> {
    let start = if total_duration <= 0.0 {
        0.0
    } else {
        (frame.time / total_duration * 100.0).clamp(0.0, 100.0)
    };
    let end = if total_duration <= 0.0 {
        100.0
    } else {
        (next_frame_time / total_duration * 100.0).clamp(start, 100.0)
    };
    writeln!(
        svg,
        r#"<g class="frame" style="animation-name: frame-{};">"#,
        index
    )?;
    writeln!(
        svg,
        r#"<style>@keyframes frame-{} {{ 0%, {:.3}% {{ opacity: 0; }} {:.3}%, {:.3}% {{ opacity: 1; }} {:.3}%, 100% {{ opacity: 0; }} }}</style>"#,
        index, start, start, end, end
    )?;
    writeln!(
        svg,
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
        layout.frame_x,
        layout.frame_y,
        layout.terminal_width,
        layout.terminal_height,
        theme.terminal.background
    )?;

    let prompt = statusline_config.unwrap_or(&theme.prompt);
    let mut y_offset: f32 = 0.0;
    let mut statusline_drawn = false;

    for row_index in 0..frame.buffer.height {
        let row = frame.buffer.row(row_index);
        let row_y = layout.frame_y + y_offset;

        if statusline && statusline::is_statusline_row(row) {
            // Only draw the bespoke statusline once per frame (on the first
            // statusline row). Subsequent statusline rows are simply skipped.
            if !statusline_drawn {
                statusline::render_bespoke_statusline(
                    svg,
                    prompt,
                    layout.frame_x,
                    row_y,
                    layout.terminal_width,
                    layout.line_height,
                    &theme.terminal.background,
                )?;
                statusline_drawn = true;
                y_offset += layout.line_height;
            }
            // Render any typed command text on its own line below the statusline
            let (cmd_start, cmd_end) = statusline::command_area(row, &theme.terminal.background);
            let cmd_y = layout.frame_y + y_offset;
            append_row_text_range(svg, layout, theme, cmd_y, row, cmd_start, cmd_end)?;
            y_offset += layout.line_height;
        } else {
            append_row_text(svg, layout, theme, row_y, row, statusline)?;
            y_offset += layout.line_height;
        }
    }
    svg.push_str("</g>");
    Ok(())
}

fn append_row_text(
    svg: &mut String,
    layout: &Layout,
    theme: &ThemeDefinition,
    row_y: f32,
    row: &[ScreenCell],
    statusline: bool,
) -> Result<()> {
    let text_y = row_y + layout.line_height * 0.14;
    for (column, cell) in row.iter().enumerate() {
        if cell.is_wide_continuation || cell.text == " " {
            continue;
        }

        let cell_x = layout.frame_x + column as f32 * layout.cell_width;
        let x = cell_x + layout.cell_width * 0.37;
        let background = effective_background(cell);
        if !background.eq_ignore_ascii_case(&theme.terminal.background) {
            writeln!(
                svg,
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                cell_x,
                row_y,
                if cell.is_wide {
                    layout.cell_width * 2.0
                } else {
                    layout.cell_width
                },
                layout.line_height,
                background
            )?;
        }

        if is_prompt_marker_glyph(&cell.text) {
            append_prompt_marker(svg, layout, row_y, column, cell)?;
            continue;
        }

        // When statusline mode is enabled, skip any Private Use Area glyph
        // so we never depend on Nerd Fonts being installed.
        if statusline && statusline::is_private_use_area(&cell.text) {
            continue;
        }

        writeln!(
            svg,
            r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}"{}>{}</text>"#,
            x,
            text_y,
            effective_foreground(cell),
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
                text_y + layout.line_height * 0.68,
                x + layout.cell_width * if cell.is_wide { 2.0 } else { 1.0 },
                text_y + layout.line_height * 0.68,
                effective_foreground(cell)
            )?;
        }
    }
    Ok(())
}

/// Render command text from a statusline row's command area.
///
/// Draws a `$` prompt marker at the left edge, then the command text
/// immediately after it, shifted to the start of the line.
fn append_row_text_range(
    svg: &mut String,
    layout: &Layout,
    theme: &ThemeDefinition,
    row_y: f32,
    row: &[ScreenCell],
    start_col: usize,
    end_col: usize,
) -> Result<()> {
    // Find the first non-space, non-PUA cell to skip leading whitespace.
    let range = &row[start_col..end_col.min(row.len())];
    let first_visible = range.iter().position(|c| {
        !c.is_wide_continuation && c.text.trim() != "" && !statusline::is_private_use_area(&c.text)
    });
    let first_visible = match first_visible {
        Some(i) => i,
        None => return Ok(()),
    };

    let text_y = row_y + layout.line_height * 0.14;
    let mut x = layout.frame_x + layout.cell_width * 0.37;

    // Draw $ prompt marker
    writeln!(
        svg,
        r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}">$</text>"#,
        x,
        text_y + layout.line_height * 0.07,
        theme.terminal.foreground
    )?;
    x += layout.cell_width * 2.0;

    // Draw command text sequentially from the left, preserving spaces
    for cell in &range[first_visible..] {
        if cell.is_wide_continuation {
            continue;
        }
        if statusline::is_private_use_area(&cell.text) || is_prompt_marker_glyph(&cell.text) {
            continue;
        }
        if cell.text == " " {
            x += layout.cell_width;
            continue;
        }

        writeln!(
            svg,
            r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}"{}>{}</text>"#,
            x,
            text_y,
            effective_foreground(cell),
            if cell.italic {
                r#" font-style="italic""#
            } else {
                ""
            },
            escape_xml(&cell.text)
        )?;
        x += if cell.is_wide {
            layout.cell_width * 2.0
        } else {
            layout.cell_width
        };
    }
    Ok(())
}

fn append_prompt_marker(
    svg: &mut String,
    layout: &Layout,
    row_y: f32,
    column: usize,
    cell: &ScreenCell,
) -> Result<()> {
    let x = layout.frame_x + column as f32 * layout.cell_width + layout.cell_width * 0.09;
    let y = row_y + layout.line_height * 0.07;
    writeln!(
        svg,
        r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}">$</text>"#,
        x,
        y,
        effective_foreground(cell)
    )?;
    Ok(())
}

fn is_prompt_marker_glyph(text: &str) -> bool {
    text.contains('')
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
                statusline: true,
                statusline_config: None,
            },
        )
        .unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("demo"));
        assert!(svg.contains(r#"<rect x="16.00" y="62.00""#));
    }

    #[test]
    fn clears_terminal_viewport_for_each_frame() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = RecordingSession::read_from_str(
            r#"{"version":2,"width":20,"height":4,"timestamp":0}
[0.1,"o","hello"]
[0.2,"o","\r\nworld"]
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
                statusline: true,
                statusline_config: None,
            },
        )
        .unwrap();
        let terminal_rect =
            r##"<rect x="16.00" y="62.00" width="216.00" height="112.00" fill="#232744"/>"##;
        assert!(svg.matches(terminal_rect).count() >= 3);
    }

    #[test]
    fn renders_bespoke_statusline_for_statusline_rows() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = RecordingSession::read_from_str(
            r#"{"version":2,"width":40,"height":4,"timestamp":0}
[0.1,"o","\u001b[38;2;214;93;14m\u001b[48;2;214;93;14;38;2;251;241;199mtest \u001b[48;2;215;153;33;38;2;214;93;14m\u001b[38;2;251;241;199m ~ \r\n"]
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
                statusline: true,
                statusline_config: None,
            },
        )
        .unwrap();
        // Bespoke statusline draws rects and polygons from theme segments
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<polygon"));
        // Bespoke segment text from theme (not from cast content)
        assert!(svg.contains("user"));
        assert!(svg.contains(">~</text>"));
    }

    #[test]
    fn respects_no_statusline_flag() {
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
                statusline: false,
                statusline_config: None,
            },
        )
        .unwrap();
        assert!(svg.contains(""));
    }

    #[test]
    fn replaces_prompt_marker_glyph_with_dollar() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = RecordingSession::read_from_str(
            r#"{"version":2,"width":20,"height":4,"timestamp":0}
[0.1,"o","\u001b[1;38;2;65;255;0m\u001b[0m echo\r\n"]
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
                statusline: true,
                statusline_config: None,
            },
        )
        .unwrap();
        assert!(svg.contains("$</text>"));
        assert!(!svg.contains(""));
    }
}
