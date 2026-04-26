pub mod ansi_parser;
pub mod screen_buffer;

use crate::cast::{EventKind, RecordingSession};
use crate::theme::ThemeDefinition;
use ansi_parser::AnsiParser;
use screen_buffer::ScreenBuffer;

#[derive(Debug, Clone)]
pub struct TerminalFrame {
    pub time: f64,
    pub buffer: ScreenBuffer,
}

pub struct TerminalEmulator {
    parser: AnsiParser,
    buffer: ScreenBuffer,
}

impl TerminalEmulator {
    pub fn new(width: usize, height: usize, theme: &ThemeDefinition, verbose: bool) -> Self {
        let buffer = ScreenBuffer::new(width, height, theme);
        let parser = AnsiParser::new(theme.clone(), verbose);
        Self { parser, buffer }
    }

    pub fn replay(&mut self, session: &RecordingSession) -> Vec<TerminalFrame> {
        let mut frames = Vec::with_capacity(session.events.len());
        for event in &session.events {
            match event.kind {
                EventKind::Output => self.parser.process(&event.data, &mut self.buffer),
                EventKind::Resize => {
                    if let Some((cols, rows)) = parse_resize(&event.data) {
                        self.buffer.resize(cols, rows);
                    }
                }
            }
            frames.push(TerminalFrame {
                time: event.time,
                buffer: self.buffer.clone(),
            });
        }
        if frames.is_empty() {
            frames.push(TerminalFrame {
                time: 0.0,
                buffer: self.buffer.clone(),
            });
        }
        frames
    }
}

/// Parse a resize event payload like `"80x24"` into `(cols, rows)`.
fn parse_resize(data: &str) -> Option<(usize, usize)> {
    let (cols, rows) = data.split_once('x')?;
    let cols = cols.trim().parse().ok()?;
    let rows = rows.trim().parse().ok()?;
    Some((cols, rows))
}
