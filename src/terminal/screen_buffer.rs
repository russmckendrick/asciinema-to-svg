use crate::theme::ThemeDefinition;

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub foreground: String,
    pub background: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reversed: bool,
    pub faint: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenCell {
    pub text: String,
    pub foreground: String,
    pub background: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reversed: bool,
    pub faint: bool,
    pub is_wide: bool,
    pub is_wide_continuation: bool,
}

impl ScreenCell {
    pub fn blank(style: &TextStyle) -> Self {
        Self {
            text: " ".to_string(),
            foreground: style.foreground.clone(),
            background: style.background.clone(),
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            reversed: style.reversed,
            faint: style.faint,
            is_wide: false,
            is_wide_continuation: false,
        }
    }

    pub fn from_text(text: &str, style: &TextStyle, wide: bool, continuation: bool) -> Self {
        Self {
            text: text.to_string(),
            foreground: style.foreground.clone(),
            background: style.background.clone(),
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            reversed: style.reversed,
            faint: style.faint,
            is_wide: wide,
            is_wide_continuation: continuation,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScreenBuffer {
    pub width: usize,
    pub height: usize,
    default_style: TextStyle,
    cells: Vec<Vec<ScreenCell>>,
    cursor_row: usize,
    cursor_col: usize,
    saved_row: usize,
    saved_col: usize,
    pending_wrap: bool,
}

impl ScreenBuffer {
    pub fn new(width: usize, height: usize, theme: &ThemeDefinition) -> Self {
        let default_style = TextStyle {
            foreground: theme.terminal.foreground.clone(),
            background: theme.terminal.background.clone(),
            bold: false,
            italic: false,
            underline: false,
            reversed: false,
            faint: false,
        };
        let cells = (0..height.max(1))
            .map(|_| {
                (0..width.max(1))
                    .map(|_| ScreenCell::blank(&default_style))
                    .collect()
            })
            .collect();

        Self {
            width: width.max(1),
            height: height.max(1),
            default_style,
            cells,
            cursor_row: 0,
            cursor_col: 0,
            saved_row: 0,
            saved_col: 0,
            pending_wrap: false,
        }
    }

    pub fn default_style(&self) -> &TextStyle {
        &self.default_style
    }

    pub fn row(&self, row: usize) -> &[ScreenCell] {
        &self.cells[row]
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    #[allow(dead_code)]
    pub fn get_cell(&self, row: usize, col: usize) -> &ScreenCell {
        &self.cells[row][col]
    }

    pub fn put_char(&mut self, ch: char, style: &TextStyle) {
        match ch {
            '\n' => self.line_feed(),
            '\r' => self.carriage_return(),
            '\x08' => self.backspace(),
            '\t' => {
                let next_stop = ((self.cursor_col / 8) + 1) * 8;
                let spaces = (next_stop.saturating_sub(self.cursor_col)).max(1);
                for _ in 0..spaces {
                    self.put_printable(" ", style);
                }
            }
            c if c.is_control() => {}
            c => self.put_printable(&c.to_string(), style),
        }
    }

    pub fn move_cursor_to(&mut self, row: usize, col: usize) {
        self.pending_wrap = false;
        self.cursor_row = row.min(self.height - 1);
        self.cursor_col = col.min(self.width - 1);
    }

    pub fn move_cursor_by(&mut self, row_delta: i32, col_delta: i32) {
        let row = (self.cursor_row as i32 + row_delta).clamp(0, self.height as i32 - 1) as usize;
        let col = (self.cursor_col as i32 + col_delta).clamp(0, self.width as i32 - 1) as usize;
        self.move_cursor_to(row, col);
    }

    pub fn carriage_return(&mut self) {
        self.pending_wrap = false;
        self.cursor_col = 0;
    }

    pub fn line_feed(&mut self) {
        self.pending_wrap = false;
        if self.cursor_row + 1 >= self.height {
            self.scroll_up(1);
        } else {
            self.cursor_row += 1;
        }
    }

    pub fn backspace(&mut self) {
        self.pending_wrap = false;
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    pub fn save_cursor(&mut self) {
        self.saved_row = self.cursor_row;
        self.saved_col = self.cursor_col;
    }

    pub fn restore_cursor(&mut self) {
        self.move_cursor_to(self.saved_row, self.saved_col);
    }

    pub fn clear_line(&mut self, mode: i32, style: Option<&TextStyle>) {
        let style = style.unwrap_or(&self.default_style).clone();
        match mode {
            1 => {
                for col in 0..=self.cursor_col {
                    self.cells[self.cursor_row][col] = ScreenCell::blank(&style);
                }
            }
            2 => {
                for col in 0..self.width {
                    self.cells[self.cursor_row][col] = ScreenCell::blank(&style);
                }
            }
            _ => {
                for col in self.cursor_col..self.width {
                    self.cells[self.cursor_row][col] = ScreenCell::blank(&style);
                }
            }
        }
    }

    pub fn clear_display(&mut self, mode: i32, style: Option<&TextStyle>) {
        let style = style.unwrap_or(&self.default_style).clone();
        match mode {
            1 => {
                for row in 0..=self.cursor_row {
                    let end = if row == self.cursor_row {
                        self.cursor_col
                    } else {
                        self.width - 1
                    };
                    for col in 0..=end {
                        self.cells[row][col] = ScreenCell::blank(&style);
                    }
                }
            }
            2 => {
                for row in 0..self.height {
                    for col in 0..self.width {
                        self.cells[row][col] = ScreenCell::blank(&style);
                    }
                }
            }
            _ => {
                for row in self.cursor_row..self.height {
                    let start = if row == self.cursor_row {
                        self.cursor_col
                    } else {
                        0
                    };
                    for col in start..self.width {
                        self.cells[row][col] = ScreenCell::blank(&style);
                    }
                }
            }
        }
    }

    pub fn delete_characters(&mut self, count: usize, style: Option<&TextStyle>) {
        let style = style.unwrap_or(&self.default_style).clone();
        let count = count.min(self.width.saturating_sub(self.cursor_col));
        let row = self.cursor_row;
        for col in self.cursor_col..self.width.saturating_sub(count) {
            self.cells[row][col] = self.cells[row][col + count].clone();
        }
        for col in self.width.saturating_sub(count)..self.width {
            self.cells[row][col] = ScreenCell::blank(&style);
        }
    }

    pub fn erase_chars(&mut self, count: usize, style: Option<&TextStyle>) {
        let style = style.unwrap_or(&self.default_style).clone();
        let end = (self.cursor_col + count).min(self.width);
        for col in self.cursor_col..end {
            self.cells[self.cursor_row][col] = ScreenCell::blank(&style);
        }
    }

    pub fn append_to_previous_cell(&mut self, text: &str) {
        if self.cursor_row == 0 && self.cursor_col == 0 {
            return;
        }
        let (row, col) = if self.cursor_col > 0 {
            (self.cursor_row, self.cursor_col - 1)
        } else {
            (self.cursor_row - 1, self.width - 1)
        };
        let target = if self.cells[row][col].is_wide_continuation && col > 0 {
            col - 1
        } else {
            col
        };
        if self.cells[row][target].text == " " {
            return;
        }
        self.cells[row][target].text.push_str(text);
    }

    fn put_printable(&mut self, text: &str, style: &TextStyle) {
        if self.pending_wrap {
            self.pending_wrap = false;
            self.cursor_col = 0;
            self.line_feed();
        }

        let wide = is_wide_character(text);
        if wide && self.cursor_col + 1 >= self.width {
            self.cursor_col = 0;
            self.line_feed();
        }

        self.cells[self.cursor_row][self.cursor_col] =
            ScreenCell::from_text(text, style, wide, false);
        self.cursor_col += 1;

        if wide && self.cursor_col < self.width {
            self.cells[self.cursor_row][self.cursor_col] =
                ScreenCell::from_text(" ", style, false, true);
            self.cursor_col += 1;
        }

        if self.cursor_col >= self.width {
            self.cursor_col = self.width - 1;
            self.pending_wrap = true;
        }
    }

    fn scroll_up(&mut self, count: usize) {
        for _ in 0..count {
            self.cells.remove(0);
            self.cells.push(
                (0..self.width)
                    .map(|_| ScreenCell::blank(&self.default_style))
                    .collect(),
            );
        }
    }
}

pub fn is_wide_character(text: &str) -> bool {
    let Some(first) = text.chars().next() else {
        return false;
    };
    let code = first as u32;
    matches!(
        code,
        0x1100..=0x115F
            | 0x2E80..=0x2FFD
            | 0x3000..=0x303F
            | 0x3040..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFF01..=0xFF60
            | 0x1F300..=0x1FAFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeDefinition;

    #[test]
    fn supports_wide_characters() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(4, 2, &theme);
        let style = buffer.default_style().clone();
        buffer.put_char('中', &style);
        assert_eq!(buffer.get_cell(0, 0).text, "中");
        assert!(buffer.get_cell(0, 0).is_wide);
        assert!(buffer.get_cell(0, 1).is_wide_continuation);
    }

    #[test]
    fn scrolls_when_newlines_overflow() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(4, 2, &theme);
        let style = buffer.default_style().clone();
        for line in ["1", "2", "3"] {
            for ch in line.chars() {
                buffer.put_char(ch, &style);
            }
            buffer.put_char('\n', &style);
        }
        assert_eq!(buffer.height, 2);
        assert!(
            buffer.row(0).iter().any(|cell| cell.text == "3")
                || buffer.row(1).iter().any(|cell| cell.text == "3")
        );
    }
}
