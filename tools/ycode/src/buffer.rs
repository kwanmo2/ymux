/// A simple text buffer with undo/redo support.
/// Uses a Vec<String> for lines — adequate for an MVP editor.

#[derive(Debug, Clone)]
pub struct Buffer {
    pub lines: Vec<String>,
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    pub dirty: bool,
}

#[derive(Debug, Clone)]
struct Snapshot {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }

    pub fn from_text(text: &str) -> Self {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(String::from).collect()
        };
        // Ensure at least one line
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        Self {
            lines,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, row: usize) -> &str {
        self.lines.get(row).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn line_len(&self, row: usize) -> usize {
        self.lines.get(row).map(|s| s.len()).unwrap_or(0)
    }

    fn save_undo(&mut self, cursor_row: usize, cursor_col: usize) {
        self.undo_stack.push(Snapshot {
            lines: self.lines.clone(),
            cursor_row,
            cursor_col,
        });
        self.redo_stack.clear();
        if self.undo_stack.len() > 1000 {
            self.undo_stack.remove(0);
        }
    }

    pub fn insert_char(&mut self, row: usize, col: usize, c: char) {
        self.save_undo(row, col);
        if row < self.lines.len() {
            let col = col.min(self.lines[row].len());
            self.lines[row].insert(col, c);
        }
        self.dirty = true;
    }

    pub fn insert_newline(&mut self, row: usize, col: usize) {
        self.save_undo(row, col);
        if row < self.lines.len() {
            let col = col.min(self.lines[row].len());
            let rest = self.lines[row][col..].to_string();
            self.lines[row].truncate(col);
            self.lines.insert(row + 1, rest);
        }
        self.dirty = true;
    }

    pub fn backspace(&mut self, row: usize, col: usize) -> (usize, usize) {
        self.save_undo(row, col);
        if col > 0 && row < self.lines.len() {
            let col = col.min(self.lines[row].len());
            self.lines[row].remove(col - 1);
            self.dirty = true;
            (row, col - 1)
        } else if row > 0 {
            let prev_len = self.lines[row - 1].len();
            let current = self.lines.remove(row);
            self.lines[row - 1].push_str(&current);
            self.dirty = true;
            (row - 1, prev_len)
        } else {
            (row, col)
        }
    }

    pub fn delete_char(&mut self, row: usize, col: usize) {
        self.save_undo(row, col);
        if row < self.lines.len() && col < self.lines[row].len() {
            self.lines[row].remove(col);
            self.dirty = true;
        } else if row < self.lines.len() - 1 && col >= self.lines[row].len() {
            let next = self.lines.remove(row + 1);
            self.lines[row].push_str(&next);
            self.dirty = true;
        }
    }

    pub fn insert_tab(&mut self, row: usize, col: usize) {
        self.save_undo(row, col);
        if row < self.lines.len() {
            let col = col.min(self.lines[row].len());
            self.lines[row].insert_str(col, "    ");
        }
        self.dirty = true;
    }

    pub fn undo(&mut self, cursor_row: usize, cursor_col: usize) -> Option<(usize, usize)> {
        if let Some(snap) = self.undo_stack.pop() {
            self.redo_stack.push(Snapshot {
                lines: self.lines.clone(),
                cursor_row,
                cursor_col,
            });
            self.lines = snap.lines;
            self.dirty = true;
            Some((snap.cursor_row, snap.cursor_col))
        } else {
            None
        }
    }

    pub fn redo(&mut self, cursor_row: usize, cursor_col: usize) -> Option<(usize, usize)> {
        if let Some(snap) = self.redo_stack.pop() {
            self.undo_stack.push(Snapshot {
                lines: self.lines.clone(),
                cursor_row,
                cursor_col,
            });
            self.lines = snap.lines;
            self.dirty = true;
            Some((snap.cursor_row, snap.cursor_col))
        } else {
            None
        }
    }

    pub fn content(&self) -> String {
        self.lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_has_one_empty_line() {
        let buf = Buffer::new();
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), "");
    }

    #[test]
    fn from_text_splits_lines() {
        let buf = Buffer::from_text("hello\nworld");
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.line(0), "hello");
        assert_eq!(buf.line(1), "world");
    }

    #[test]
    fn from_empty_text() {
        let buf = Buffer::from_text("");
        assert_eq!(buf.line_count(), 1);
    }

    #[test]
    fn insert_char_basic() {
        let mut buf = Buffer::from_text("hello");
        buf.insert_char(0, 5, '!');
        assert_eq!(buf.line(0), "hello!");
        assert!(buf.dirty);
    }

    #[test]
    fn insert_char_middle() {
        let mut buf = Buffer::from_text("hllo");
        buf.insert_char(0, 1, 'e');
        assert_eq!(buf.line(0), "hello");
    }

    #[test]
    fn insert_newline_splits_line() {
        let mut buf = Buffer::from_text("helloworld");
        buf.insert_newline(0, 5);
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.line(0), "hello");
        assert_eq!(buf.line(1), "world");
    }

    #[test]
    fn backspace_within_line() {
        let mut buf = Buffer::from_text("hello");
        let (r, c) = buf.backspace(0, 5);
        assert_eq!((r, c), (0, 4));
        assert_eq!(buf.line(0), "hell");
    }

    #[test]
    fn backspace_joins_lines() {
        let mut buf = Buffer::from_text("hello\nworld");
        let (r, c) = buf.backspace(1, 0);
        assert_eq!((r, c), (0, 5));
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), "helloworld");
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut buf = Buffer::from_text("hello");
        let (r, c) = buf.backspace(0, 0);
        assert_eq!((r, c), (0, 0));
        assert_eq!(buf.line(0), "hello");
    }

    #[test]
    fn delete_char_basic() {
        let mut buf = Buffer::from_text("hello");
        buf.delete_char(0, 0);
        assert_eq!(buf.line(0), "ello");
    }

    #[test]
    fn delete_char_at_eol_joins() {
        let mut buf = Buffer::from_text("hello\nworld");
        buf.delete_char(0, 5);
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), "helloworld");
    }

    #[test]
    fn insert_tab() {
        let mut buf = Buffer::from_text("hello");
        buf.insert_tab(0, 0);
        assert_eq!(buf.line(0), "    hello");
    }

    #[test]
    fn undo_redo() {
        let mut buf = Buffer::from_text("hello");
        buf.insert_char(0, 5, '!');
        assert_eq!(buf.line(0), "hello!");

        let pos = buf.undo(0, 6);
        assert!(pos.is_some());
        assert_eq!(buf.line(0), "hello");

        let pos = buf.redo(0, 5);
        assert!(pos.is_some());
        assert_eq!(buf.line(0), "hello!");
    }

    #[test]
    fn undo_clears_redo_on_edit() {
        let mut buf = Buffer::from_text("hello");
        buf.insert_char(0, 5, '!');
        buf.undo(0, 6);
        buf.insert_char(0, 5, '?');
        assert!(buf.redo(0, 6).is_none());
    }

    #[test]
    fn to_string_roundtrip() {
        let text = "line1\nline2\nline3";
        let buf = Buffer::from_text(text);
        assert_eq!(buf.content(), text);
    }
}
