mod statusline;

use crate::cast::RecordingSession;
use crate::terminal::{TerminalEmulator, TerminalFrame, screen_buffer::ScreenCell};
use crate::theme::{ChromeKind, PromptTheme, ThemeDefinition};
use anyhow::Result;
use std::fmt::Write;

pub struct RenderOptions {
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
    /// Explicit `--title` override (highest priority).
    pub window_title: Option<String>,
    /// Title to use when `window_title` is not set and the cast did not set
    /// an OSC 0/2 title (typically the input file stem).
    pub fallback_title: Option<String>,
    pub statusline: bool,
    pub statusline_config: Option<PromptTheme>,

    /// Playback speed multiplier (1.0 = real time).
    pub speed: f32,
    /// Cap any inter-frame gap at this many seconds.
    pub idle_time_limit: Option<f32>,
    /// Trim the cast to `[start, end]` (seconds, original timeline).
    pub start: Option<f64>,
    pub end: Option<f64>,
    /// If set, render a single static frame at this time and skip animation.
    pub at: Option<f64>,
    /// True (default) loops the animation forever; false plays once.
    pub loop_animation: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width_px: None,
            height_px: None,
            window_title: None,
            fallback_title: None,
            statusline: true,
            statusline_config: None,
            speed: 1.0,
            idle_time_limit: None,
            start: None,
            end: None,
            at: None,
            loop_animation: true,
        }
    }
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
    let all_frames = emulator.replay(session);
    let mut frames = deduplicate_frames(all_frames);
    apply_timing(&mut frames, &options);
    if let Some(at) = options.at {
        frames = select_static_frame(frames, at);
    }
    normalize_frame_timing(&mut frames);
    let static_mode = options.at.is_some();

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
                    if statusline::is_statusline_row(row) && first {
                        // First statusline row gets an extra line for itself
                        extra += 1;
                        first = false;
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

    // CLI override wins; otherwise prefer the OSC 0/2 title captured during
    // replay (most recent non-empty title across frames), then the caller's
    // fallback (typically the cast file stem), then a generic label.
    let title = options.window_title.clone().unwrap_or_else(|| {
        frames
            .iter()
            .rev()
            .find_map(|f| {
                f.buffer
                    .title()
                    .filter(|t| !t.is_empty())
                    .map(str::to_string)
            })
            .or_else(|| options.fallback_title.clone())
            .unwrap_or_else(|| "Terminal".to_string())
    });
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
    append_styles(
        &mut svg,
        theme,
        total_duration,
        static_mode,
        options.loop_animation,
    )?;
    svg.push_str("</defs>");
    append_window_chrome(&mut svg, theme, &layout, &title)?;
    writeln!(
        svg,
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
        layout.frame_x.round(),
        layout.frame_y.round(),
        layout.terminal_width.round(),
        layout.terminal_height.round(),
        theme.terminal.background
    )?;

    if static_mode {
        // Single frame, no animation wrapper, no @keyframes.
        if let Some(frame) = frames.first() {
            append_frame_body(
                &mut svg,
                theme,
                &layout,
                frame,
                options.statusline,
                options.statusline_config.as_ref(),
            )?;
        }
    } else {
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
    }

    svg.push_str("</svg>");
    Ok(svg)
}

fn append_styles(
    svg: &mut String,
    theme: &ThemeDefinition,
    duration: f64,
    static_mode: bool,
    loop_animation: bool,
) -> Result<()> {
    let frame_rule = if static_mode {
        String::new()
    } else {
        let iter = if loop_animation { "infinite" } else { "1" };
        format!(
            r#"
        .frame {{
            opacity: 0;
            animation-duration: {}s;
            animation-timing-function: steps(1, end);
            animation-iteration-count: {};
            will-change: opacity;
            transform: translateZ(0);
        }}"#,
            duration, iter
        )
    };
    writeln!(
        svg,
        r#"<style>
        .terminal-text {{
            font-family: {};
            font-size: {}px;
            font-weight: 400;
            dominant-baseline: hanging;
            white-space: pre;
        }}{}
        </style>"#,
        css_text(&theme.font_family),
        theme.font_size,
        frame_rule
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
    // Chrome glyphs and circles are emitted once, but the browser may still
    // re-rasterize the chrome layer on every animation cycle. Snap their
    // coordinates to whole pixels so long titles can't shimmer via sub-pixel
    // glyph repositioning.
    let title_x = match theme.chrome.kind {
        ChromeKind::Macos | ChromeKind::Linux => (layout.width / 2.0).round(),
        ChromeKind::Powershell => 12.0,
    };
    let title_y = (title_bar_center_y + 0.5).round();
    writeln!(
        svg,
        r#"<text x="{:.2}" y="{:.2}" font-family="{}" font-size="{:.1}" fill="{}" dominant-baseline="middle"{}>{}</text>"#,
        title_x,
        title_y,
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
            let cy = (title_bar_center_y + 0.5).round();
            for (index, color) in ["#ff5f57", "#febc2e", "#28c840"].iter().enumerate() {
                writeln!(
                    svg,
                    r#"<circle cx="{:.2}" cy="{:.2}" r="7" fill="{}"/>"#,
                    (theme.chrome.padding + 18.0 + 22.0 * index as f32).round(),
                    cy,
                    color
                )?;
            }
        }
        ChromeKind::Linux => {
            let y = (theme.chrome.padding + theme.chrome.title_bar_height / 2.0).round();
            writeln!(
                svg,
                r##"<circle cx="18" cy="{:.2}" r="8" fill="#dd4814"/><circle cx="42" cy="{:.2}" r="8" fill="#666666"/><circle cx="66" cy="{:.2}" r="8" fill="#888888"/>"##,
                y, y, y
            )?;
        }
        ChromeKind::Powershell => {
            let y = (theme.chrome.padding + theme.chrome.title_bar_height / 2.0).round();
            let ctrl_font = theme.chrome.title_bar_height * 0.3;
            writeln!(
                svg,
                r#"<text x="{:.2}" y="{:.2}" font-family="Segoe UI, sans-serif" font-size="{:.1}" fill="{}" dominant-baseline="middle">_</text>"#,
                (layout.width - 70.0).round(),
                y,
                ctrl_font,
                theme.chrome.subtitle_color
            )?;
            writeln!(
                svg,
                r#"<rect x="{:.2}" y="{:.2}" width="10" height="10" fill="none" stroke="{}"/>"#,
                (layout.width - 46.0).round(),
                (y - 5.0).round(),
                theme.chrome.subtitle_color
            )?;
            writeln!(
                svg,
                r#"<text x="{:.2}" y="{:.2}" font-family="Segoe UI, sans-serif" font-size="{:.1}" fill="{}" dominant-baseline="middle">×</text>"#,
                (layout.width - 18.0).round(),
                y,
                ctrl_font,
                theme.chrome.subtitle_color
            )?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
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
    append_frame_body(svg, theme, layout, frame, statusline, statusline_config)?;
    svg.push_str("</g>");
    Ok(())
}

/// Emit the visual contents of a single frame: the per-frame terminal-bg fill
/// followed by per-row content (statusline rendering or plain row text).
///
/// Used both inside the per-frame `<g>` wrapper for animated mode and as the
/// sole rendered content for static (`--at`) mode.
fn append_frame_body(
    svg: &mut String,
    theme: &ThemeDefinition,
    layout: &Layout,
    frame: &TerminalFrame,
    statusline: bool,
    statusline_config: Option<&PromptTheme>,
) -> Result<()> {
    writeln!(
        svg,
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
        layout.frame_x.round(),
        layout.frame_y.round(),
        layout.terminal_width.round(),
        layout.terminal_height.round(),
        theme.terminal.background
    )?;

    let prompt = statusline_config.unwrap_or(&theme.prompt);
    let mut y_offset: f32 = 0.0;
    let mut statusline_drawn = false;

    for row_index in 0..frame.buffer.height {
        let row = frame.buffer.row(row_index);
        let row_y = (layout.frame_y + y_offset).round();

        if statusline && statusline::is_statusline_row(row) {
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
                let (cmd_start, cmd_end) =
                    statusline::command_area(row, &theme.terminal.background);
                let cmd_y = (layout.frame_y + y_offset).round();
                append_row_text_range(svg, layout, theme, cmd_y, row, cmd_start, cmd_end)?;
                y_offset += layout.line_height;
            } else {
                statusline::render_dynamic_statusline(
                    svg,
                    row,
                    layout.frame_x,
                    row_y,
                    layout.terminal_width,
                    layout.line_height,
                    layout.cell_width,
                    &theme.terminal.background,
                    &theme.font_family,
                    theme.font_size,
                )?;
                y_offset += layout.line_height;
            }
        } else {
            append_row_text(svg, layout, theme, row_y, row, statusline)?;
            y_offset += layout.line_height;
        }
    }
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
    // All emitted coordinates are snapped to whole pixels so Safari (and other
    // browsers using GPU compositor layers per <g>) can re-rasterize each
    // frame's layer to identical pixels every animation cycle. Sub-pixel
    // coordinates re-rasterize with slight variance, producing visible shimmer.
    let row_y = row_y.round();
    let line_h = layout.line_height.round();
    let text_y = (row_y + layout.line_height * 0.14).round();
    for (column, cell) in row.iter().enumerate() {
        if cell.is_wide_continuation || cell.text == " " {
            continue;
        }

        let cell_cols = if cell.is_wide { 2 } else { 1 };
        let cell_x = (layout.frame_x + column as f32 * layout.cell_width).round();
        let cell_right = (layout.frame_x + (column + cell_cols) as f32 * layout.cell_width).round();
        let cell_w = cell_right - cell_x;
        let x = (cell_x + layout.cell_width * 0.37).round();
        let background = effective_background(cell);
        if !background.eq_ignore_ascii_case(&theme.terminal.background) {
            writeln!(
                svg,
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                cell_x, row_y, cell_w, line_h, background
            )?;
        }

        if is_prompt_marker_glyph(&cell.text) {
            append_prompt_marker(
                svg,
                layout,
                row_y,
                column,
                cell,
                &theme.prompt.trailing_symbol,
            )?;
            continue;
        }

        // Render block element characters as SVG rects instead of text glyphs
        // for pixel-perfect rendering regardless of font support. Edges snap
        // to the same integer pixel grid as the cell itself so half-blocks
        // (▀ ▄ ▌ ▐) tile cleanly without seams.
        if let Some(regions) = block_char_regions(&cell.text) {
            let fg = effective_foreground(cell);
            let cw_f = layout.cell_width;
            let ch_f = layout.line_height;
            for (rx, ry, rw, rh) in regions {
                let x0 = (cell_x + rx * cw_f).round();
                let y0 = (row_y + ry * ch_f).round();
                let x1 = (cell_x + (rx + rw) * cw_f).round();
                let y1 = (row_y + (ry + rh) * ch_f).round();
                writeln!(
                    svg,
                    r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                    x0,
                    y0,
                    x1 - x0,
                    y1 - y0,
                    fg
                )?;
            }
            continue;
        }

        // When statusline mode is enabled, skip any Private Use Area glyph
        // so we never depend on Nerd Fonts being installed.
        if statusline && statusline::is_private_use_area(&cell.text) {
            continue;
        }

        let mut extra_attrs = String::new();
        if cell.bold {
            extra_attrs.push_str(r#" font-weight="bold""#);
        }
        if cell.italic {
            extra_attrs.push_str(r#" font-style="italic""#);
        }
        if cell.faint {
            extra_attrs.push_str(r#" opacity="0.5""#);
        }

        writeln!(
            svg,
            r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}"{}>{}</text>"#,
            x,
            text_y,
            effective_foreground(cell),
            extra_attrs,
            escape_xml(&cell.text)
        )?;

        let fg = effective_foreground(cell);
        if cell.underline {
            let uy = (text_y + layout.line_height * 0.68).round();
            writeln!(
                svg,
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.2"/>"#,
                x,
                uy,
                x + cell_w,
                uy,
                fg
            )?;
        }
        if cell.strikethrough {
            let sy = (text_y + layout.line_height * 0.25).round();
            writeln!(
                svg,
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.2"/>"#,
                x,
                sy,
                x + cell_w,
                sy,
                fg
            )?;
        }
        if cell.overline {
            writeln!(
                svg,
                r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{}" stroke-width="1.2"/>"#,
                cell_x,
                row_y,
                cell_x + cell_w,
                row_y,
                fg
            )?;
        }
    }
    Ok(())
}

/// Render command text from a statusline row's command area.
///
/// Draws the theme's `prompt.trailing_symbol` (e.g. `$` on Linux, `❯` on
/// macOS, `>` on PowerShell) at the left edge, then the command text
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

    let row_y = row_y.round();
    let text_y = (row_y + layout.line_height * 0.14).round();
    let prompt_y = (row_y + layout.line_height * 0.14 + layout.line_height * 0.07).round();
    let mut x = layout.frame_x + layout.cell_width * 0.37;

    // Draw the theme's prompt prefix glyph.
    writeln!(
        svg,
        r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}">{}</text>"#,
        x.round(),
        prompt_y,
        theme.terminal.foreground,
        escape_xml(&theme.prompt.trailing_symbol)
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
            x.round(),
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
    glyph: &str,
) -> Result<()> {
    let x = (layout.frame_x + column as f32 * layout.cell_width + layout.cell_width * 0.09).round();
    let y = (row_y + layout.line_height * 0.07).round();
    writeln!(
        svg,
        r#"<text class="terminal-text" x="{:.2}" y="{:.2}" fill="{}">{}</text>"#,
        x,
        y,
        effective_foreground(cell),
        escape_xml(glyph)
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

/// Apply the timing transformations driven by CLI flags, in order:
///   1. Trim to `[start, end]` and rebase the surviving frames to start at 0.
///   2. Cap any inter-frame gap that exceeds `idle_time_limit`, sliding all
///      subsequent frames earlier by the saved time.
///   3. Multiply all times by `1 / speed` so faster speeds compress the cast.
///
/// The order matters: trim defines the slice the user cares about, idle-cap
/// then makes that slice watchable by removing dead air, and speed finally
/// scales the whole thing. Operations are no-ops when their flag is unset.
fn apply_timing(frames: &mut Vec<TerminalFrame>, opts: &RenderOptions) {
    if let Some(start) = opts.start {
        frames.retain(|f| f.time >= start);
        for f in frames.iter_mut() {
            f.time -= start;
        }
    }
    if let Some(end) = opts.end {
        let cutoff = end - opts.start.unwrap_or(0.0);
        frames.retain(|f| f.time <= cutoff);
    }

    if let Some(limit) = opts.idle_time_limit.filter(|l| *l > 0.0) {
        let limit = limit as f64;
        let mut total_saved = 0.0_f64;
        let mut last_original: Option<f64> = None;
        for f in frames.iter_mut() {
            let original = f.time;
            f.time = original - total_saved;
            if let Some(prev) = last_original {
                let gap = original - prev;
                if gap > limit {
                    let saving = gap - limit;
                    total_saved += saving;
                    f.time -= saving;
                }
            }
            last_original = Some(original);
        }
    }

    if (opts.speed - 1.0).abs() > 1e-6 && opts.speed > 0.0 {
        let scale = 1.0 / opts.speed as f64;
        for f in frames.iter_mut() {
            f.time *= scale;
        }
    }
}

/// Pick the buffer state at time `at` for `--at` static export.
///
/// Returns the last frame whose time is <= `at`. If `at` is before any frame,
/// returns the first frame (the empty initial buffer); if all frames are
/// before `at`, returns the final frame.
fn select_static_frame(frames: Vec<TerminalFrame>, at: f64) -> Vec<TerminalFrame> {
    if frames.is_empty() {
        return frames;
    }
    let mut chosen_index = 0usize;
    for (i, frame) in frames.iter().enumerate() {
        if frame.time <= at {
            chosen_index = i;
        } else {
            break;
        }
    }
    let mut iter = frames.into_iter();
    let chosen = iter.nth(chosen_index).expect("non-empty by guard above");
    vec![chosen]
}

/// Remove consecutive frames whose visible buffer content is identical,
/// keeping only the last frame in each run of duplicates (to preserve timing).
fn deduplicate_frames(frames: Vec<TerminalFrame>) -> Vec<TerminalFrame> {
    if frames.len() <= 1 {
        return frames;
    }
    let mut result: Vec<TerminalFrame> = Vec::with_capacity(frames.len());
    for frame in frames {
        if let Some(prev) = result.last()
            && buffers_equal(&prev.buffer, &frame.buffer)
        {
            // Replace previous with this one (keep later timestamp)
            let last = result.len() - 1;
            result[last] = frame;
            continue;
        }
        result.push(frame);
    }
    result
}

fn buffers_equal(
    a: &crate::terminal::screen_buffer::ScreenBuffer,
    b: &crate::terminal::screen_buffer::ScreenBuffer,
) -> bool {
    if a.width != b.width || a.height != b.height {
        return false;
    }
    for row in 0..a.height {
        let ra = a.row(row);
        let rb = b.row(row);
        for col in 0..a.width {
            if ra[col] != rb[col] {
                return false;
            }
        }
    }
    true
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

/// Map Unicode block element characters to sub-cell rectangle regions.
/// Returns (x_frac, y_frac, w_frac, h_frac) tuples relative to cell size.
fn block_char_regions(text: &str) -> Option<Vec<(f32, f32, f32, f32)>> {
    let ch = text.chars().next()?;
    let regions = match ch {
        // Full block
        '█' => vec![(0.0, 0.0, 1.0, 1.0)],
        // Half blocks
        '▀' => vec![(0.0, 0.0, 1.0, 0.5)],
        '▄' => vec![(0.0, 0.5, 1.0, 0.5)],
        '▌' => vec![(0.0, 0.0, 0.5, 1.0)],
        '▐' => vec![(0.5, 0.0, 0.5, 1.0)],
        // Quadrant blocks
        '▘' => vec![(0.0, 0.0, 0.5, 0.5)],
        '▝' => vec![(0.5, 0.0, 0.5, 0.5)],
        '▖' => vec![(0.0, 0.5, 0.5, 0.5)],
        '▗' => vec![(0.5, 0.5, 0.5, 0.5)],
        // Three-quadrant blocks
        '▛' => vec![(0.0, 0.0, 1.0, 0.5), (0.0, 0.5, 0.5, 0.5)],
        '▜' => vec![(0.0, 0.0, 1.0, 0.5), (0.5, 0.5, 0.5, 0.5)],
        '▙' => vec![(0.0, 0.0, 0.5, 0.5), (0.0, 0.5, 1.0, 0.5)],
        '▟' => vec![(0.5, 0.0, 0.5, 0.5), (0.0, 0.5, 1.0, 0.5)],
        // Partial vertical blocks (left side, increasing width)
        '▏' => vec![(0.0, 0.0, 0.125, 1.0)],
        '▎' => vec![(0.0, 0.0, 0.25, 1.0)],
        '▍' => vec![(0.0, 0.0, 0.375, 1.0)],
        '▋' => vec![(0.0, 0.0, 0.625, 1.0)],
        '▊' => vec![(0.0, 0.0, 0.75, 1.0)],
        '▉' => vec![(0.0, 0.0, 0.875, 1.0)],
        // Partial horizontal blocks (bottom, increasing height)
        '▁' => vec![(0.0, 0.875, 1.0, 0.125)],
        '▂' => vec![(0.0, 0.75, 1.0, 0.25)],
        '▃' => vec![(0.0, 0.625, 1.0, 0.375)],
        '▅' => vec![(0.0, 0.375, 1.0, 0.625)],
        '▆' => vec![(0.0, 0.25, 1.0, 0.75)],
        '▇' => vec![(0.0, 0.125, 1.0, 0.875)],
        // Right partial blocks
        '▕' => vec![(0.875, 0.0, 0.125, 1.0)],
        // Box-drawing: horizontal lines rendered as thin rects for seamless tiling
        '─' | '━' => vec![(0.0, 0.45, 1.0, 0.1)],
        '│' | '┃' => vec![(0.45, 0.0, 0.1, 1.0)],
        // Light box-drawing corners and tees
        '┌' => vec![(0.45, 0.45, 0.55, 0.1), (0.45, 0.45, 0.1, 0.55)],
        '┐' => vec![(0.0, 0.45, 0.55, 0.1), (0.45, 0.45, 0.1, 0.55)],
        '└' => vec![(0.45, 0.0, 0.1, 0.55), (0.45, 0.45, 0.55, 0.1)],
        '┘' => vec![(0.45, 0.0, 0.1, 0.55), (0.0, 0.45, 0.55, 0.1)],
        '├' => vec![(0.45, 0.0, 0.1, 1.0), (0.45, 0.45, 0.55, 0.1)],
        '┤' => vec![(0.45, 0.0, 0.1, 1.0), (0.0, 0.45, 0.55, 0.1)],
        '┬' => vec![(0.0, 0.45, 1.0, 0.1), (0.45, 0.45, 0.1, 0.55)],
        '┴' => vec![(0.0, 0.45, 1.0, 0.1), (0.45, 0.0, 0.1, 0.55)],
        '┼' => vec![(0.0, 0.45, 1.0, 0.1), (0.45, 0.0, 0.1, 1.0)],
        _ => return None,
    };
    Some(regions)
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
            },
        )
        .unwrap();
        assert!(svg.contains(""));
    }

    #[test]
    fn replaces_prompt_marker_glyph_with_theme_trailing_symbol() {
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
                ..Default::default()
            },
        )
        .unwrap();
        // macOS theme defines trailing_symbol as `❯` (U+276F).
        assert!(svg.contains("❯</text>"));
        assert!(!svg.contains(""));
    }

    fn make_simple_session(events: &[(f64, &str)]) -> RecordingSession {
        let mut content = String::from(r#"{"version":2,"width":20,"height":4,"timestamp":0}"#);
        for (time, data) in events {
            content.push('\n');
            content.push_str(&format!(
                r#"[{},"o",{}]"#,
                time,
                serde_json::to_string(data).unwrap()
            ));
        }
        RecordingSession::read_from_str(&content).unwrap()
    }

    fn frames_at_times(times: &[f64]) -> Vec<TerminalFrame> {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        times
            .iter()
            .map(|&t| TerminalFrame {
                time: t,
                buffer: crate::terminal::screen_buffer::ScreenBuffer::new(2, 2, &theme),
            })
            .collect()
    }

    #[test]
    fn apply_timing_speed_halves_frame_times() {
        let mut frames = frames_at_times(&[0.0, 1.0, 2.0]);
        let opts = RenderOptions {
            speed: 2.0,
            ..Default::default()
        };
        apply_timing(&mut frames, &opts);
        assert!((frames[0].time - 0.0).abs() < 1e-9);
        assert!((frames[1].time - 0.5).abs() < 1e-9);
        assert!((frames[2].time - 1.0).abs() < 1e-9);
    }

    #[test]
    fn apply_timing_idle_cap_collapses_long_pauses() {
        let mut frames = frames_at_times(&[0.0, 0.1, 5.1, 5.2]);
        let opts = RenderOptions {
            idle_time_limit: Some(0.5),
            ..Default::default()
        };
        apply_timing(&mut frames, &opts);
        assert!((frames[0].time - 0.0).abs() < 1e-9);
        assert!((frames[1].time - 0.1).abs() < 1e-9);
        assert!((frames[2].time - 0.6).abs() < 1e-9);
        assert!((frames[3].time - 0.7).abs() < 1e-9);
    }

    #[test]
    fn apply_timing_start_end_trims_and_rebases() {
        let mut frames = frames_at_times(&[0.0, 1.0, 2.0, 3.0, 4.0]);
        let opts = RenderOptions {
            start: Some(1.0),
            end: Some(3.0),
            ..Default::default()
        };
        apply_timing(&mut frames, &opts);
        assert_eq!(frames.len(), 3);
        assert!((frames[0].time - 0.0).abs() < 1e-9);
        assert!((frames[1].time - 1.0).abs() < 1e-9);
        assert!((frames[2].time - 2.0).abs() < 1e-9);
    }

    #[test]
    fn at_produces_static_svg_with_no_animation() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = make_simple_session(&[(0.1, "first"), (0.5, "\r\nsecond")]);
        let svg = render_animated_svg(
            &session,
            &theme,
            RenderOptions {
                at: Some(0.6),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            !svg.contains("@keyframes"),
            "static SVG must not contain @keyframes"
        );
        assert!(
            !svg.contains(r#"class="frame""#),
            "static SVG must not wrap content in a .frame group"
        );
        assert!(svg.contains(">f</text>"));
        assert!(svg.contains(">s</text>"));
    }

    #[test]
    fn no_loop_sets_iteration_count_to_one() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = make_simple_session(&[(0.1, "hi")]);
        let svg = render_animated_svg(
            &session,
            &theme,
            RenderOptions {
                loop_animation: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(svg.contains("animation-iteration-count: 1;"));
        assert!(!svg.contains("animation-iteration-count: infinite"));
    }

    #[test]
    fn osc_title_used_when_no_cli_title() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = make_simple_session(&[(0.1, "\u{1b}]2;set-by-osc\u{07}hello")]);
        let svg = render_animated_svg(
            &session,
            &theme,
            RenderOptions {
                window_title: None,
                fallback_title: Some("file-stem".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(svg.contains(">set-by-osc</text>"));
        assert!(!svg.contains(">file-stem</text>"));
    }

    #[test]
    fn fallback_title_used_when_no_cli_or_osc_title() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let session = make_simple_session(&[(0.1, "no title set")]);
        let svg = render_animated_svg(
            &session,
            &theme,
            RenderOptions {
                window_title: None,
                fallback_title: Some("file-stem".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(svg.contains(">file-stem</text>"));
    }
}
