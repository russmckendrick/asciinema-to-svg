use super::screen_buffer::{ScreenBuffer, TextStyle};
use crate::theme::ThemeDefinition;

pub struct AnsiParser {
    style: TextStyle,
    theme: ThemeDefinition,
    pending_escape: String,
}

impl AnsiParser {
    pub fn new(style: TextStyle, theme: ThemeDefinition) -> Self {
        Self {
            style,
            theme,
            pending_escape: String::new(),
        }
    }

    pub fn process(&mut self, text: &str, buffer: &mut ScreenBuffer) {
        if text.is_empty() {
            return;
        }

        let combined = if self.pending_escape.is_empty() {
            text.to_string()
        } else {
            let merged = format!("{}{}", self.pending_escape, text);
            self.pending_escape.clear();
            merged
        };

        let chars: Vec<char> = combined.chars().collect();
        let mut index = 0usize;

        while index < chars.len() {
            let ch = chars[index];
            if ch == '\x1b' {
                match self.try_handle_escape(&chars, index, buffer) {
                    Some(next) => {
                        index = next + 1;
                        continue;
                    }
                    None => {
                        self.pending_escape = chars[index..].iter().collect();
                        break;
                    }
                }
            }

            if is_variation_selector(ch) || is_combining_mark(ch) {
                buffer.append_to_previous_cell(&ch.to_string());
                index += 1;
                continue;
            }
            if is_zero_width_char(ch) {
                index += 1;
                continue;
            }

            buffer.put_char(ch, &self.style);
            index += 1;
        }
    }

    fn try_handle_escape(
        &mut self,
        chars: &[char],
        index: usize,
        buffer: &mut ScreenBuffer,
    ) -> Option<usize> {
        if index + 1 >= chars.len() {
            return None;
        }
        match chars[index + 1] {
            '[' => self.try_handle_csi(chars, index + 2, buffer),
            ']' => skip_osc(chars, index + 2),
            '7' => {
                buffer.save_cursor();
                Some(index + 1)
            }
            '8' => {
                buffer.restore_cursor();
                Some(index + 1)
            }
            'D' => {
                buffer.line_feed();
                Some(index + 1)
            }
            'M' => {
                buffer.reverse_index();
                Some(index + 1)
            }
            'c' => {
                self.style = buffer.default_style().clone();
                buffer.clear_display(2, None);
                buffer.move_cursor_to(0, 0);
                Some(index + 1)
            }
            // Character set selection (skip the designator byte)
            '(' | ')' | '*' | '+' => {
                if index + 2 < chars.len() {
                    Some(index + 2)
                } else {
                    None
                }
            }
            _ => Some(index + 1),
        }
    }

    fn try_handle_csi(
        &mut self,
        chars: &[char],
        start: usize,
        buffer: &mut ScreenBuffer,
    ) -> Option<usize> {
        let mut private_marker = None;
        let mut param_start = start;
        if start < chars.len() && matches!(chars[start], '<' | '=' | '>' | '?') {
            private_marker = Some(chars[start]);
            param_start += 1;
        }

        for index in param_start..chars.len() {
            let ch = chars[index];
            if ('@'..='~').contains(&ch) {
                let params =
                    parse_parameters(&chars[param_start..index].iter().collect::<String>());
                if private_marker == Some('?') {
                    self.apply_private_mode(ch, &params, buffer);
                } else if private_marker.is_none() {
                    self.apply_csi(ch, &params, buffer);
                }
                return Some(index);
            }
        }
        None
    }

    fn apply_csi(&mut self, command: char, parameters: &[i32], buffer: &mut ScreenBuffer) {
        match command {
            'm' => self.apply_sgr(parameters, buffer),
            'A' => buffer.move_cursor_by(-get_param(parameters, 0, 1), 0),
            'B' | 'e' => buffer.move_cursor_by(get_param(parameters, 0, 1), 0),
            'C' | 'a' => buffer.move_cursor_by(0, get_param(parameters, 0, 1)),
            'D' => buffer.move_cursor_by(0, -get_param(parameters, 0, 1)),
            'E' => {
                buffer.move_cursor_by(get_param(parameters, 0, 1), 0);
                buffer.carriage_return();
            }
            'F' => {
                buffer.move_cursor_by(-get_param(parameters, 0, 1), 0);
                buffer.carriage_return();
            }
            'G' | '`' => buffer.move_cursor_to(
                buffer.cursor_row(),
                (get_param(parameters, 0, 1).max(1) - 1) as usize,
            ),
            'H' | 'f' => {
                let row = (get_param(parameters, 0, 1).max(1) - 1) as usize;
                let col = (get_param(parameters, 1, 1).max(1) - 1) as usize;
                buffer.move_cursor_to(row, col);
            }
            'J' => buffer.clear_display(get_param(parameters, 0, 0), Some(&self.style)),
            'K' => buffer.clear_line(get_param(parameters, 0, 0), Some(&self.style)),
            'P' => buffer.delete_characters(
                get_param(parameters, 0, 1).max(1) as usize,
                Some(&self.style),
            ),
            'X' => buffer.erase_chars(
                get_param(parameters, 0, 1).max(1) as usize,
                Some(&self.style),
            ),
            'L' => buffer.insert_lines(get_param(parameters, 0, 1).max(1) as usize),
            'M' => buffer.delete_lines(get_param(parameters, 0, 1).max(1) as usize),
            '@' => buffer.insert_characters(get_param(parameters, 0, 1).max(1) as usize),
            'r' => {
                let top = (get_param(parameters, 0, 1).max(1) - 1) as usize;
                let bottom = (get_param(parameters, 1, buffer.height() as i32).max(1) - 1) as usize;
                buffer.set_scroll_region(top, bottom);
            }
            's' => buffer.save_cursor(),
            'u' => buffer.restore_cursor(),
            _ => {}
        }
    }

    fn apply_private_mode(&mut self, command: char, parameters: &[i32], buffer: &mut ScreenBuffer) {
        match command {
            'h' => {
                for &param in parameters {
                    match param {
                        1049 => buffer.enter_alt_screen(),
                        _ => {}
                    }
                }
            }
            'l' => {
                for &param in parameters {
                    match param {
                        1049 => buffer.exit_alt_screen(),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn apply_sgr(&mut self, parameters: &[i32], buffer: &mut ScreenBuffer) {
        let parameters = if parameters.is_empty() {
            &[0][..]
        } else {
            parameters
        };

        let mut index = 0usize;
        while index < parameters.len() {
            match parameters[index] {
                0 => self.style = buffer.default_style().clone(),
                1 => {
                    self.style.bold = true;
                    self.style.faint = false;
                }
                2 => {
                    self.style.bold = false;
                    self.style.faint = true;
                }
                3 => self.style.italic = true,
                4 => self.style.underline = true,
                7 => self.style.reversed = true,
                9 => self.style.strikethrough = true,
                22 => {
                    self.style.bold = false;
                    self.style.faint = false;
                }
                23 => self.style.italic = false,
                24 => self.style.underline = false,
                27 => self.style.reversed = false,
                29 => self.style.strikethrough = false,
                53 => self.style.overline = true,
                55 => self.style.overline = false,
                39 => self.style.foreground = buffer.default_style().foreground.clone(),
                49 => self.style.background = buffer.default_style().background.clone(),
                30..=37 => {
                    self.style.foreground = self
                        .theme
                        .ansi_color((parameters[index] - 30) as usize)
                        .to_string()
                }
                40..=47 => {
                    self.style.background = self
                        .theme
                        .ansi_color((parameters[index] - 40) as usize)
                        .to_string()
                }
                90..=97 => {
                    self.style.foreground = self
                        .theme
                        .ansi_color((8 + parameters[index] - 90) as usize)
                        .to_string()
                }
                100..=107 => {
                    self.style.background = self
                        .theme
                        .ansi_color((8 + parameters[index] - 100) as usize)
                        .to_string()
                }
                38 | 48 => {
                    if index + 2 < parameters.len() && parameters[index + 1] == 5 {
                        let color = self
                            .theme
                            .ansi256_color(parameters[index + 2].clamp(0, 255) as u8);
                        if parameters[index] == 38 {
                            self.style.foreground = color;
                        } else {
                            self.style.background = color;
                        }
                        index += 2;
                    } else if index + 4 < parameters.len() && parameters[index + 1] == 2 {
                        let color = format!(
                            "#{:02X}{:02X}{:02X}",
                            parameters[index + 2].clamp(0, 255) as u8,
                            parameters[index + 3].clamp(0, 255) as u8,
                            parameters[index + 4].clamp(0, 255) as u8
                        );
                        if parameters[index] == 38 {
                            self.style.foreground = color;
                        } else {
                            self.style.background = color;
                        }
                        index += 4;
                    }
                }
                _ => {}
            }
            index += 1;
        }
    }
}

fn parse_parameters(value: &str) -> Vec<i32> {
    if value.is_empty() {
        return Vec::new();
    }
    value
        .split(';')
        .map(|part| {
            if part.is_empty() {
                0
            } else {
                part.parse().unwrap_or(0)
            }
        })
        .collect()
}

fn get_param(parameters: &[i32], index: usize, default: i32) -> i32 {
    parameters.get(index).copied().unwrap_or(default)
}

fn skip_osc(chars: &[char], start: usize) -> Option<usize> {
    for index in start..chars.len() {
        if chars[index] == '\x07' {
            return Some(index);
        }
        if chars[index] == '\x1b' && index + 1 < chars.len() && chars[index + 1] == '\\' {
            return Some(index + 1);
        }
    }
    None
}

fn is_zero_width_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{00AD}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}'
    )
}

fn is_variation_selector(ch: char) -> bool {
    ('\u{FE00}'..='\u{FE0F}').contains(&ch)
}

fn is_combining_mark(ch: char) -> bool {
    let cp = ch as u32;
    (0x0300..=0x036F).contains(&cp)
        || (0x1AB0..=0x1AFF).contains(&cp)
        || (0x20D0..=0x20FF).contains(&cp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeDefinition;

    #[test]
    fn applies_ansi_colors() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(8, 2, &theme);
        let style = buffer.default_style().clone();
        let mut parser = AnsiParser::new(style, theme);
        parser.process("\x1b[31mA\x1b[0mB", &mut buffer);
        assert_eq!(buffer.get_cell(0, 0).foreground, "#ff6b6b");
        assert_eq!(
            buffer.get_cell(0, 1).foreground,
            buffer.default_style().foreground
        );
    }

    #[test]
    fn moves_cursor_left() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(8, 2, &theme);
        let style = buffer.default_style().clone();
        let mut parser = AnsiParser::new(style, theme);
        parser.process("ABC\x1b[1D!", &mut buffer);
        assert_eq!(buffer.get_cell(0, 2).text, "!");
    }

    #[test]
    fn applies_bold_and_faint() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(8, 2, &theme);
        let style = buffer.default_style().clone();
        let mut parser = AnsiParser::new(style, theme);
        parser.process("\x1b[1mB\x1b[2mF\x1b[22mN", &mut buffer);
        assert!(buffer.get_cell(0, 0).bold);
        assert!(!buffer.get_cell(0, 0).faint);
        assert!(buffer.get_cell(0, 1).faint);
        assert!(!buffer.get_cell(0, 1).bold);
        assert!(!buffer.get_cell(0, 2).bold);
        assert!(!buffer.get_cell(0, 2).faint);
    }

    #[test]
    fn applies_strikethrough() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(8, 2, &theme);
        let style = buffer.default_style().clone();
        let mut parser = AnsiParser::new(style, theme);
        parser.process("\x1b[9mS\x1b[29mN", &mut buffer);
        assert!(buffer.get_cell(0, 0).strikethrough);
        assert!(!buffer.get_cell(0, 1).strikethrough);
    }

    #[test]
    fn applies_overline() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(8, 2, &theme);
        let style = buffer.default_style().clone();
        let mut parser = AnsiParser::new(style, theme);
        parser.process("\x1b[53mO\x1b[55mN", &mut buffer);
        assert!(buffer.get_cell(0, 0).overline);
        assert!(!buffer.get_cell(0, 1).overline);
    }

    #[test]
    fn handles_scroll_region() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(10, 5, &theme);
        let style = buffer.default_style().clone();
        let mut parser = AnsiParser::new(style, theme);
        // Set scroll region to rows 2-4 (1-indexed: 2;4)
        parser.process("\x1b[2;4r", &mut buffer);
        // Cursor should be at 0,0 after setting scroll region
        assert_eq!(buffer.cursor_row(), 0);
    }

    #[test]
    fn handles_insert_delete_lines() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(4, 4, &theme);
        let style = buffer.default_style().clone();
        let mut parser = AnsiParser::new(style, theme);
        parser.process("AAA\n", &mut buffer);
        parser.process("BBB\n", &mut buffer);
        parser.process("CCC", &mut buffer);
        // Move to row 1 and insert a line
        parser.process("\x1b[2;1H\x1b[1L", &mut buffer);
        // Row 1 should now be blank
        assert_eq!(buffer.get_cell(1, 0).text, " ");
    }

    #[test]
    fn handles_alt_screen() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(4, 2, &theme);
        let style = buffer.default_style().clone();
        let mut parser = AnsiParser::new(style, theme);
        parser.process("ABCD", &mut buffer);
        assert_eq!(buffer.get_cell(0, 0).text, "A");
        // Enter alt screen
        parser.process("\x1b[?1049h", &mut buffer);
        assert_eq!(buffer.get_cell(0, 0).text, " ");
        parser.process("XY", &mut buffer);
        assert_eq!(buffer.get_cell(0, 0).text, "X");
        // Exit alt screen
        parser.process("\x1b[?1049l", &mut buffer);
        assert_eq!(buffer.get_cell(0, 0).text, "A");
    }
}
