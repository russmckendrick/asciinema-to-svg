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
    pub strikethrough: bool,
    pub overline: bool,
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
    pub strikethrough: bool,
    pub overline: bool,
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
            strikethrough: style.strikethrough,
            overline: style.overline,
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
            strikethrough: style.strikethrough,
            overline: style.overline,
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
    /// The active text style applied to incoming glyphs and to cells cleared
    /// by ED/EL/DCH/ECH. The parser mutates this directly via SGR, instead of
    /// holding its own parallel `TextStyle` that could drift out of sync.
    current_style: TextStyle,
    cells: Vec<Vec<ScreenCell>>,
    cursor_row: usize,
    cursor_col: usize,
    cursor_visible: bool,
    saved_row: usize,
    saved_col: usize,
    pending_wrap: bool,
    scroll_top: usize,
    scroll_bottom: usize,
    alt_cells: Option<Vec<Vec<ScreenCell>>>,
    alt_cursor_row: usize,
    alt_cursor_col: usize,
    title: Option<String>,
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
            strikethrough: false,
            overline: false,
        };
        let cells = (0..height.max(1))
            .map(|_| {
                (0..width.max(1))
                    .map(|_| ScreenCell::blank(&default_style))
                    .collect()
            })
            .collect();

        let h = height.max(1);
        Self {
            width: width.max(1),
            height: h,
            current_style: default_style.clone(),
            default_style,
            cells,
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
            saved_row: 0,
            saved_col: 0,
            pending_wrap: false,
            scroll_top: 0,
            scroll_bottom: h - 1,
            alt_cells: None,
            alt_cursor_row: 0,
            alt_cursor_col: 0,
            title: None,
        }
    }

    #[allow(dead_code)]
    pub fn current_style(&self) -> &TextStyle {
        &self.current_style
    }

    pub fn current_style_mut(&mut self) -> &mut TextStyle {
        &mut self.current_style
    }

    pub fn reset_current_style(&mut self) {
        self.current_style = self.default_style.clone();
    }

    // Wired by the parser via ?25h/l; the renderer will consume this once
    // theme-driven cursor rendering lands.
    #[allow(dead_code)]
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn set_title(&mut self, title: String) {
        self.title = Some(title);
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

    pub fn height(&self) -> usize {
        self.height
    }

    #[allow(dead_code)]
    pub fn get_cell(&self, row: usize, col: usize) -> &ScreenCell {
        &self.cells[row][col]
    }

    pub fn put_char(&mut self, ch: char) {
        match ch {
            '\n' => self.line_feed(),
            '\r' => self.carriage_return(),
            '\x08' => self.backspace(),
            '\t' => {
                let next_stop = ((self.cursor_col / 8) + 1) * 8;
                let spaces = (next_stop.saturating_sub(self.cursor_col)).max(1);
                for _ in 0..spaces {
                    self.put_printable(" ");
                }
            }
            c if c.is_control() => {}
            c => self.put_printable(&c.to_string()),
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
        if self.cursor_row == self.scroll_bottom {
            self.scroll_up_region(1);
        } else if self.cursor_row + 1 < self.height {
            self.cursor_row += 1;
        }
    }

    pub fn reverse_index(&mut self) {
        self.pending_wrap = false;
        if self.cursor_row == self.scroll_top {
            self.scroll_down_region(1);
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
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

    pub fn clear_line(&mut self, mode: i32) {
        let style = self.current_style.clone();
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

    pub fn clear_display(&mut self, mode: i32) {
        let style = self.current_style.clone();
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

    pub fn delete_characters(&mut self, count: usize) {
        let style = self.current_style.clone();
        let count = count.min(self.width.saturating_sub(self.cursor_col));
        let row = self.cursor_row;
        for col in self.cursor_col..self.width.saturating_sub(count) {
            self.cells[row][col] = self.cells[row][col + count].clone();
        }
        for col in self.width.saturating_sub(count)..self.width {
            self.cells[row][col] = ScreenCell::blank(&style);
        }
    }

    pub fn erase_chars(&mut self, count: usize) {
        let style = self.current_style.clone();
        let end = (self.cursor_col + count).min(self.width);
        for col in self.cursor_col..end {
            self.cells[self.cursor_row][col] = ScreenCell::blank(&style);
        }
    }

    pub fn append_to_previous_cell(&mut self, text: &str) {
        // At buffer start with no preceding cell, attach to cell (0,0) itself
        // if it has visible content; otherwise drop (no glyph to combine onto).
        if self.cursor_row == 0 && self.cursor_col == 0 {
            if self.cells[0][0].text != " " {
                self.cells[0][0].text.push_str(text);
            }
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

    fn put_printable(&mut self, text: &str) {
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
            ScreenCell::from_text(text, &self.current_style, wide, false);
        self.cursor_col += 1;

        if wide && self.cursor_col < self.width {
            self.cells[self.cursor_row][self.cursor_col] =
                ScreenCell::from_text(" ", &self.current_style, false, true);
            self.cursor_col += 1;
        }

        if self.cursor_col >= self.width {
            self.cursor_col = self.width - 1;
            self.pending_wrap = true;
        }
    }

    fn scroll_up_region(&mut self, count: usize) {
        let top = self.scroll_top;
        let bottom = self.scroll_bottom;
        for _ in 0..count {
            if top < self.cells.len() {
                self.cells.remove(top);
                let blank_row: Vec<ScreenCell> = (0..self.width)
                    .map(|_| ScreenCell::blank(&self.default_style))
                    .collect();
                let insert_at = bottom.min(self.cells.len());
                self.cells.insert(insert_at, blank_row);
            }
        }
    }

    fn scroll_down_region(&mut self, count: usize) {
        let top = self.scroll_top;
        let bottom = self.scroll_bottom;
        for _ in 0..count {
            if bottom < self.cells.len() {
                self.cells.remove(bottom);
                let blank_row: Vec<ScreenCell> = (0..self.width)
                    .map(|_| ScreenCell::blank(&self.default_style))
                    .collect();
                self.cells.insert(top, blank_row);
            }
        }
    }

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        self.scroll_top = top.min(self.height - 1);
        self.scroll_bottom = bottom.min(self.height - 1);
        if self.scroll_top > self.scroll_bottom {
            std::mem::swap(&mut self.scroll_top, &mut self.scroll_bottom);
        }
        self.move_cursor_to(0, 0);
    }

    pub fn insert_lines(&mut self, count: usize) {
        let row = self.cursor_row;
        if row < self.scroll_top || row > self.scroll_bottom {
            return;
        }
        for _ in 0..count {
            if self.scroll_bottom < self.cells.len() {
                self.cells.remove(self.scroll_bottom);
            }
            let blank_row: Vec<ScreenCell> = (0..self.width)
                .map(|_| ScreenCell::blank(&self.default_style))
                .collect();
            self.cells.insert(row, blank_row);
        }
        self.cursor_col = 0;
    }

    pub fn delete_lines(&mut self, count: usize) {
        let row = self.cursor_row;
        if row < self.scroll_top || row > self.scroll_bottom {
            return;
        }
        for _ in 0..count {
            if row < self.cells.len() {
                self.cells.remove(row);
            }
            let blank_row: Vec<ScreenCell> = (0..self.width)
                .map(|_| ScreenCell::blank(&self.default_style))
                .collect();
            let insert_at = self.scroll_bottom.min(self.cells.len());
            self.cells.insert(insert_at, blank_row);
        }
        self.cursor_col = 0;
    }

    pub fn insert_characters(&mut self, count: usize) {
        let row = self.cursor_row;
        let col = self.cursor_col;
        let count = count.min(self.width.saturating_sub(col));
        for _ in 0..count {
            if self.cells[row].len() > self.width.saturating_sub(1) {
                self.cells[row].pop();
            }
            self.cells[row].insert(col, ScreenCell::blank(&self.default_style));
        }
        // Ensure row length stays correct
        self.cells[row].truncate(self.width);
    }

    /// Resize the buffer in place, preserving as much of the existing content
    /// as fits. Cells become blank (in the current style) when the buffer
    /// grows; rows and columns past the new bounds are truncated when it
    /// shrinks. Cursor and scroll region are clamped to the new geometry.
    pub fn resize(&mut self, width: usize, height: usize) {
        let new_w = width.max(1);
        let new_h = height.max(1);
        let blank = ScreenCell::blank(&self.current_style);

        for row in self.cells.iter_mut() {
            if row.len() < new_w {
                row.resize(new_w, blank.clone());
            } else {
                row.truncate(new_w);
            }
        }
        if self.cells.len() < new_h {
            let blank_row: Vec<ScreenCell> = (0..new_w).map(|_| blank.clone()).collect();
            while self.cells.len() < new_h {
                self.cells.push(blank_row.clone());
            }
        } else {
            self.cells.truncate(new_h);
        }

        self.width = new_w;
        self.height = new_h;
        self.cursor_row = self.cursor_row.min(new_h - 1);
        self.cursor_col = self.cursor_col.min(new_w - 1);
        self.saved_row = self.saved_row.min(new_h - 1);
        self.saved_col = self.saved_col.min(new_w - 1);
        self.scroll_top = 0;
        self.scroll_bottom = new_h - 1;
        self.pending_wrap = false;
    }

    pub fn enter_alt_screen(&mut self) {
        let saved = self.cells.clone();
        self.alt_cells = Some(saved);
        self.alt_cursor_row = self.cursor_row;
        self.alt_cursor_col = self.cursor_col;
        // Clear the screen for the alt buffer
        for row in 0..self.height {
            for col in 0..self.width {
                self.cells[row][col] = ScreenCell::blank(&self.default_style);
            }
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.pending_wrap = false;
    }

    pub fn exit_alt_screen(&mut self) {
        if let Some(saved) = self.alt_cells.take() {
            self.cells = saved;
            self.cursor_row = self.alt_cursor_row;
            self.cursor_col = self.alt_cursor_col;
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
        buffer.put_char('中');
        assert_eq!(buffer.get_cell(0, 0).text, "中");
        assert!(buffer.get_cell(0, 0).is_wide);
        assert!(buffer.get_cell(0, 1).is_wide_continuation);
    }

    #[test]
    fn cursor_visibility_defaults_true_and_can_toggle() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(4, 2, &theme);
        assert!(buffer.cursor_visible());
        buffer.set_cursor_visible(false);
        assert!(!buffer.cursor_visible());
    }

    #[test]
    fn title_starts_none_and_can_be_set() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(4, 2, &theme);
        assert_eq!(buffer.title(), None);
        buffer.set_title("hello".to_string());
        assert_eq!(buffer.title(), Some("hello"));
    }

    #[test]
    fn combining_mark_at_buffer_start_attaches_when_cell_has_glyph() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(4, 2, &theme);
        // Place 'e' at (0,0), then move cursor back so a combining mark
        // arriving with cursor at (0,0) attaches to (0,0)'s glyph.
        buffer.put_char('e');
        buffer.move_cursor_to(0, 0);
        buffer.append_to_previous_cell("\u{0301}"); // combining acute
        assert_eq!(buffer.get_cell(0, 0).text, "e\u{0301}");
    }

    #[test]
    fn resize_grows_preserving_existing_content() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(4, 2, &theme);
        for ch in "abcd".chars() {
            buffer.put_char(ch);
        }
        buffer.resize(8, 4);
        assert_eq!(buffer.width, 8);
        assert_eq!(buffer.height, 4);
        // Original glyphs survive at their original positions.
        assert_eq!(buffer.get_cell(0, 0).text, "a");
        assert_eq!(buffer.get_cell(0, 3).text, "d");
        // New cells are blank.
        assert_eq!(buffer.get_cell(0, 4).text, " ");
        assert_eq!(buffer.get_cell(3, 0).text, " ");
    }

    #[test]
    fn resize_shrink_clamps_cursor_and_drops_overflow() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(8, 4, &theme);
        for ch in "12345678".chars() {
            buffer.put_char(ch);
        }
        // Cursor is at end of row 0.
        buffer.resize(4, 2);
        assert_eq!(buffer.width, 4);
        assert_eq!(buffer.height, 2);
        // Cursor clamped inside new bounds.
        assert!(buffer.cursor_row() < 2);
        // Visible content is the truncated head of the original row.
        assert_eq!(buffer.get_cell(0, 0).text, "1");
        assert_eq!(buffer.get_cell(0, 3).text, "4");
    }

    #[test]
    fn scrolls_when_newlines_overflow() {
        let theme = ThemeDefinition::load(Some("macos")).unwrap();
        let mut buffer = ScreenBuffer::new(4, 2, &theme);
        for line in ["1", "2", "3"] {
            for ch in line.chars() {
                buffer.put_char(ch);
            }
            buffer.put_char('\n');
        }
        assert_eq!(buffer.height, 2);
        assert!(
            buffer.row(0).iter().any(|cell| cell.text == "3")
                || buffer.row(1).iter().any(|cell| cell.text == "3")
        );
    }
}
