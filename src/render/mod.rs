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
    let natural_terminal_width = session.terminal_size.width as f32 * natural_cell_width;
    let natural_terminal_height = session.terminal_size.height as f32 * natural_line_height;
    let natural_width = theme.chrome.padding * 2.0 + natural_terminal_width;
    let natural_height =
        theme.chrome.padding * 2.0 + theme.chrome.title_bar_height + natural_terminal_height;

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
            frame_y: theme.chrome.padding + theme.chrome.title_bar_height,
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
            frame_y: theme.chrome.padding + theme.chrome.title_bar_height,
            terminal_width: (width - theme.chrome.padding * 2.0).max(1.0),
            terminal_height: (height - theme.chrome.padding * 2.0 - theme.chrome.title_bar_height)
                .max(1.0),
            cell_width: ((width - theme.chrome.padding * 2.0)
                / session.terminal_size.width.max(1) as f32)
                .max(theme.font_size * 0.52),
            line_height: ((height - theme.chrome.padding * 2.0 - theme.chrome.title_bar_height)
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
        writeln!(
            svg,
            r##"<rect x="1" y="1" width="{:.2}" height="{:.2}" rx="{:.2}" fill="#2a3157" opacity="0.82"/>"##,
            layout.width - 2.0,
            theme.chrome.title_bar_height + theme.chrome.padding * 0.6,
            (theme.chrome.radius - 2.0).max(0.0)
        )?;
        writeln!(
            svg,
            r##"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="#3a4677" stroke-width="1" opacity="0.8"/>"##,
            theme.chrome.padding,
            theme.chrome.padding + theme.chrome.title_bar_height + 0.5,
            layout.width - theme.chrome.padding,
            theme.chrome.padding + theme.chrome.title_bar_height + 0.5
        )?;
    }

    writeln!(
        svg,
        r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{}" fill="{}" dominant-baseline="middle"{}>{}</text>"#,
        match theme.chrome.kind {
            ChromeKind::Macos | ChromeKind::Linux => layout.width / 2.0,
            ChromeKind::Powershell => 12.0,
        },
        theme.chrome.padding + theme.chrome.title_bar_height / 2.0 + 0.5,
        css_text("ui-sans-serif, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"),
        if matches!(theme.chrome.kind, ChromeKind::Macos) {
            17
        } else {
            14
        },
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
                    theme.chrome.padding + theme.chrome.title_bar_height / 2.0 + 0.5,
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
    writeln!(
        svg,
        r#"<g class="frame" style="animation-name: frame-{};">"#,
        index
    )?;
    writeln!(
        svg,
        r#"<style>@keyframes frame-{} {{ 0%, {:.3}% {{ opacity: 0; }} {:.3}%, 100% {{ opacity: 1; }} }}</style>"#,
        index, start, start
    )?;

    let mut display_row_index = 0usize;
    for row_index in 0..frame.buffer.height {
        let row = frame.buffer.row(row_index);
        if powerline && should_skip_powerline_row(row) {
            continue;
        }
        append_row_text(svg, layout, theme, display_row_index, row, powerline)?;
        display_row_index += 1;
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
    powerline: bool,
) -> Result<()> {
    let text_y = layout.frame_y + row_index as f32 * layout.line_height + 4.0;
    for (column, cell) in row.iter().enumerate() {
        if cell.is_wide_continuation || cell.text == " " {
            continue;
        }

        let cell_x = layout.frame_x + column as f32 * layout.cell_width;
        let x = cell_x + 4.0;
        let background = effective_background(cell);
        if !background.eq_ignore_ascii_case(&theme.terminal.background) {
            writeln!(
                svg,
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                cell_x,
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

        if powerline && is_powerline_glyph(&cell.text) {
            append_powerline_glyph(svg, layout, row_index, column, cell)?;
            continue;
        }

        if is_prompt_marker_glyph(&cell.text) {
            append_prompt_marker(svg, layout, row_index, column, cell)?;
            continue;
        }

        writeln!(
            svg,
            r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}"{}{}>{}</text>"#,
            x,
            text_y,
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
                text_y + 19.0,
                x + layout.cell_width * if cell.is_wide { 2.0 } else { 1.0 },
                text_y + 19.0,
                effective_foreground(cell)
            )?;
        }
    }
    Ok(())
}

fn append_powerline_glyph(
    svg: &mut String,
    layout: &Layout,
    row_index: usize,
    column: usize,
    cell: &ScreenCell,
) -> Result<()> {
    let x = layout.frame_x + column as f32 * layout.cell_width;
    let y = layout.frame_y + row_index as f32 * layout.line_height;
    let w = layout.cell_width;
    let h = layout.line_height;
    let fg = effective_foreground(cell);

    match cell.text.as_str() {
        "" => {
            writeln!(
                svg,
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}"/>"#,
                x,
                y,
                x + w,
                y + h / 2.0,
                x,
                y + h,
                fg
            )?;
        }
        "" => {
            writeln!(
                svg,
                r#"<polygon points="{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}" fill="{}"/>"#,
                x + w,
                y,
                x,
                y + h / 2.0,
                x + w,
                y + h,
                fg
            )?;
        }
        "" | "" => {
            writeln!(
                svg,
                r#"<path d="M {:.2} {:.2} L {:.2} {:.2}" stroke="{}" stroke-width="2" stroke-linecap="round"/>"#,
                x + w * 0.28,
                y + 4.0,
                x + w * 0.72,
                y + h - 4.0,
                fg
            )?;
        }
        "" => {
            writeln!(
                svg,
                r#"<path d="M {:.2} {:.2} Q {:.2} {:.2} {:.2} {:.2} L {:.2} {:.2} Q {:.2} {:.2} {:.2} {:.2} Z" fill="{}"/>"#,
                x + w,
                y,
                x + w * 0.18,
                y,
                x,
                y + h / 2.0,
                x,
                y + h / 2.0,
                x + w * 0.18,
                y + h,
                x + w,
                y + h,
                fg
            )?;
        }
        _ => {}
    }

    Ok(())
}

fn is_powerline_glyph(text: &str) -> bool {
    matches!(text, "" | "" | "" | "" | "")
}

fn append_prompt_marker(
    svg: &mut String,
    layout: &Layout,
    row_index: usize,
    column: usize,
    cell: &ScreenCell,
) -> Result<()> {
    let x = layout.frame_x + column as f32 * layout.cell_width + 1.0;
    let y = layout.frame_y + row_index as f32 * layout.line_height + 2.0;
    writeln!(
        svg,
        r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}" font-weight="700">$</text>"#,
        x,
        y,
        effective_foreground(cell)
    )?;
    Ok(())
}

fn is_prompt_marker_glyph(text: &str) -> bool {
    text.contains('')
}

fn should_skip_powerline_row(row: &[ScreenCell]) -> bool {
    row.iter()
        .any(|cell| !cell.is_wide_continuation && is_powerline_glyph(&cell.text))
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
                powerline: true,
            },
        )
        .unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("demo"));
    }

    #[test]
    fn drops_powerline_rows_when_cleanup_is_enabled() {
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
                powerline: true,
            },
        )
        .unwrap();
        assert!(!svg.contains("test"));
        assert!(!svg.contains("<polygon"));
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
                powerline: true,
            },
        )
        .unwrap();
        assert!(svg.contains("$</text>"));
        assert!(!svg.contains(""));
    }
}
