//! Keyboard event handler for the interactive TUI.
//!
//! [`handle_event`] translates raw [`crossterm`] key events into mutations on
//! [`crate::tui::app::App`] state.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::tui::{
    app::{App, AppStatus},
    clipboard,
};

/// Poll for the next terminal event and apply it to `app`.
///
/// This function blocks until an event is received. Returns `true` when the
/// event loop should continue, and `false` when the application should exit.
pub fn handle_event(app: &mut App) -> anyhow::Result<bool> {
    match event::read()? {
        Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key),
        // Ignorekey-release events(Windows sends both Press and Release
        // per keystroke; Unix backends typecally only send Press) and
        // resize /mouse events.
        _ => Ok(true),
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match (key.modifiers, key.code) {
        // ── Execution ────────────────────────────────────────────────────────
        // Enter with no modifiers: confirm and start the build.
        (KeyModifiers::NONE, KeyCode::Enter) => {
            app.status = AppStatus::Confirmed;
            return Ok(false);
        }

        // Shift+Enter or Alt+Enter: insert a newline in the editor.
        (KeyModifiers::SHIFT | KeyModifiers::ALT, KeyCode::Enter) => {
            app.insert_newline();
        }

        // Escape: cancel without writing to disk.
        (KeyModifiers::NONE, KeyCode::Esc) => {
            app.status = AppStatus::Cancelled;
            return Ok(false);
        }

        // ── Clipboard ────────────────────────────────────────────────────────
        // Ctrl+V: paste text from the system clipboard.
        (KeyModifiers::CONTROL, KeyCode::Char('v')) => match clipboard::read_clipboard() {
            Ok(text) => {
                app.set_content(&text);
                app.last_error = None;
            }
            Err(e) => {
                app.last_error = Some(e.to_string());
            }
        },

        // ── Buffer management ─────────────────────────────────────────────────
        // / Ctrl+C: clear the input buffer.
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            app.status = AppStatus::Cancelled;
            return Ok(false);
        }
        // Ctrl+R stays only for clear:
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
            app.clear();
        }

        // Ctrl+E: load the built-in example tree template.
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
            const EXAMPLE: &str = include_str!("../../examples/example_tree.txt");
            app.set_content(EXAMPLE);
        }

        // ── Cursor Navigation ────────────────────────────────────────────────
        (KeyModifiers::NONE, KeyCode::Left) => app.move_cursor_left(),
        (KeyModifiers::NONE, KeyCode::Right) => app.move_cursor_right(),
        (KeyModifiers::NONE, KeyCode::Up) => app.move_cursor_up(),
        (KeyModifiers::NONE, KeyCode::Down) => app.move_cursor_down(),
        (KeyModifiers::NONE, KeyCode::Home) => app.move_cursor_home(),
        (KeyModifiers::NONE, KeyCode::End) => app.move_cursor_end(),

        // ── Text editing ──────────────────────────────────────────────────────
        (KeyModifiers::NONE, KeyCode::Backspace) => {
            app.delete_char();
        }
        (KeyModifiers::NONE, KeyCode::Delete) => {
            app.delete_forward_char();
        }
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(ch)) => {
            app.insert_char(ch);
        }

        _ => {}
    }

    Ok(true)
}
