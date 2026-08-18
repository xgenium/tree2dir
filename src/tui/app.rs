//! TUI application state.
//!
//! [`App`] is the single source of truth for the interactive editor session.
//! It holds the current input buffer, the cursor position, and the
//! [`AppStatus`] that drives the main event loop.

use crate::parser;

/// Lifecycle states of the TUI application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    /// The TUI is running and awaiting user input.
    Running,
    /// The user confirmed with `Enter`; build should proceed.
    Confirmed,
    /// The user pressed `Esc`; exit without writing to disk.
    Cancelled,
}

/// Live statistics derived from the current buffer.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LiveStats {
    /// Number of directory entries in the current parse result.
    pub dir_count: usize,
    /// Number of file entries in the current parse result.
    pub file_count: usize,
}

/// All mutable state for one interactive TUI session.
///
/// # UTF-8 Cursor Indexing Strategy
/// `cursor_col` represents the 0-based **character (Unicode scalar) index**
/// within `lines[cursor_line]`, NOT the byte index. Convert to byte offset
/// via [`App::cursor_byte_offset`] before mutating `String` buffers.
#[derive(Debug)]
pub struct App {
    /// Lines of text currently in the editor area.
    pub lines: Vec<String>,
    /// Index of the line the cursor is on (0-based).
    pub cursor_line: usize,
    /// Character offset of the cursor within the current line (0-based).
    pub cursor_col: usize,
    /// Current lifecycle state.
    pub status: AppStatus,
    /// Live parse statistics shown in the preview bar.
    pub stats: LiveStats,
    /// Title shown in the top border.
    pub title: String,
    /// The last error that will be processed and displayed.
    pub last_error: Option<String>,
}

impl App {
    /// Create a new, empty [`App`] instance.
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            status: AppStatus::Running,
            stats: LiveStats::default(),
            title: format!("TreeBuilder v{}", env!("CARGO_PKG_VERSION")),
            last_error: None,
        }
    }

    /// Calculate the byte offset corresponding to `self.cursor_col` in the current line.
    pub fn cursor_byte_offset(&self) -> usize {
        let line = &self.lines[self.cursor_line];
        line.char_indices()
            .nth(self.cursor_col)
            .map(|(idx, _)| idx)
            .unwrap_or(line.len())
    }

    /// Replace the entire buffer with `text`, splitting on newlines.
    pub fn set_content(&mut self, text: &str) {
        self.lines = text.lines().map(str::to_owned).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_line = self.lines.len().saturating_sub(1);
        self.cursor_col = self.lines[self.cursor_line].chars().count();
        self.refresh_stats();
    }

    /// Clear the input buffer and reset the cursor.
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.stats = LiveStats::default();
    }

    /// Insert a single character at the current cursor character position.
    pub fn insert_char(&mut self, ch: char) {
        let byte_idx = self.cursor_byte_offset();
        self.lines[self.cursor_line].insert(byte_idx, ch);
        self.cursor_col += 1;
        self.refresh_stats();
    }

    /// Handle the `Enter` key: split the current line at the cursor character position.
    pub fn insert_newline(&mut self) {
        let byte_idx = self.cursor_byte_offset();
        let current = self.lines[self.cursor_line].split_off(byte_idx);
        self.cursor_line += 1;
        self.lines.insert(self.cursor_line, current);
        self.cursor_col = 0;
        self.refresh_stats();
    }

    /// Delete the character immediately before the cursor (Backspace).
    pub fn delete_char(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            let byte_idx = self.cursor_byte_offset();
            self.lines[self.cursor_line].remove(byte_idx);
        } else if self.cursor_line > 0 {
            // Merge current line with the line above.
            let current = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].chars().count();
            self.lines[self.cursor_line].push_str(&current);
        }
        self.refresh_stats();
    }

    /// Delete the character immediately at the cursor (Delete key).
    pub fn delete_forward_char(&mut self) {
        let line_len = self.lines[self.cursor_line].chars().count();
        if self.cursor_col < line_len {
            let byte_idx = self.cursor_byte_offset();
            self.lines[self.cursor_line].remove(byte_idx);
        } else if self.cursor_line + 1 < self.lines.len() {
            let next_line = self.lines.remove(self.cursor_line + 1);
            self.lines[self.cursor_line].push_str(&next_line);
        }
        self.refresh_stats();
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    /// Move the cursor left by one character, wrapping to the end of the previous line if needed.
    pub fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].chars().count();
        }
    }

    /// Move the cursor right by one character, wrapping to the start of the next line if needed.
    pub fn move_cursor_right(&mut self) {
        let line_len = self.lines[self.cursor_line].chars().count();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.cursor_col = 0;
        }
    }

    /// Move the cursor up by one line.
    pub fn move_cursor_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            self.clamp_cursor_col();
        }
    }

    /// Move the cursor down by one line.
    pub fn move_cursor_down(&mut self) {
        if self.cursor_line + 1 < self.lines.len() {
            self.cursor_line += 1;
            self.clamp_cursor_col();
        }
    }

    /// Move the cursor to the beginning of the current line.
    pub fn move_cursor_home(&mut self) {
        self.cursor_col = 0;
    }

    /// Move the cursor to the end of the current line.
    pub fn move_cursor_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_line].chars().count();
    }

    /// Clamp `cursor_col` so it does not exceed the character count of `cursor_line`.
    fn clamp_cursor_col(&mut self) {
        let line_len = self.lines[self.cursor_line].chars().count();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    /// Return the full buffer text as a single `\n`-separated string.
    pub fn buffer_text(&self) -> String {
        self.lines.join("\n")
    }

    /// Re-parse the buffer and update [`Self::stats`].
    fn refresh_stats(&mut self) {
        let text = self.buffer_text();
        let nodes = parser::parse_tree(&text);
        self.stats.dir_count = nodes
            .iter()
            .filter(|n| n.kind == crate::parser::EntryKind::Dir)
            .count();
        self.stats.file_count = nodes
            .iter()
            .filter(|n| n.kind == crate::parser::EntryKind::File)
            .count();
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_character_insertion_and_deletion() {
        let mut app = App::new();

        // Insert multi-byte Unicode characters (3-byte UTF-8 box-drawing characters)
        app.insert_char('│');
        app.insert_char(' ');
        app.insert_char('├');
        app.insert_char('─');
        app.insert_char(' ');
        app.insert_char('📁'); // 4-byte emoji

        assert_eq!(app.lines[0], "│ ├─ 📁");
        assert_eq!(app.cursor_col, 6); // 6 Unicode scalars

        // Delete characters one by one with Backspace
        app.delete_char(); // deletes 📁
        assert_eq!(app.lines[0], "│ ├─ ");
        assert_eq!(app.cursor_col, 5);

        app.delete_char(); // deletes space
        assert_eq!(app.lines[0], "│ ├─");
        assert_eq!(app.cursor_col, 4);

        app.delete_char(); // deletes ─
        assert_eq!(app.lines[0], "│ ├");
        assert_eq!(app.cursor_col, 3);
    }

    #[test]
    fn test_set_content_utf8_cursor_col() {
        let mut app = App::new();
        app.set_content("├── src/\n│   └── main.rs");
        assert_eq!(app.cursor_line, 1);
        // "│   └── main.rs" has 15 characters, but 21 bytes.
        assert_eq!(app.cursor_col, 15);
    }
}
