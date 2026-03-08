pub mod ansi_parser;
pub mod screen_buffer;

use crate::cast::RecordingSession;
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
    pub fn new(width: usize, height: usize, theme: &ThemeDefinition) -> Self {
        let buffer = ScreenBuffer::new(width, height, theme);
        let parser = AnsiParser::new(buffer.default_style().clone(), theme.clone());
        Self { parser, buffer }
    }

    pub fn replay(&mut self, session: &RecordingSession) -> Vec<TerminalFrame> {
        let mut frames = Vec::with_capacity(session.events.len());
        for event in &session.events {
            self.parser.process(&event.data, &mut self.buffer);
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
