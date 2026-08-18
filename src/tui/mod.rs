//! Interactive TUI module.
//!
//! Exposes [`run_tui`], the entry point for the interactive terminal session.
//!
//! # Layout
//!
//! | Submodule | Responsibility |
//! |-----------|---------------|
//! | [`app`]   | Application state (`App`, `AppStatus`, `LiveStats`) |
//! | [`event`] | Keyboard event handler |
//! | [`ui`]    | Ratatui widget definitions and layout |
//! | [`clipboard`] | System clipboard integration via `arboard` |

pub mod app;
pub mod clipboard;
pub mod event;
pub mod ui;

use std::io;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::error::AppError;
use crate::tui::app::{App, AppStatus};

/// Launch the interactive TUI and return the user's final buffer text when
/// they confirm with `Enter`.
///
/// Returns `None` if the user pressed `Esc` (cancelled), or the buffer text
/// as a `String` if they pressed `Enter` (confirmed).
///
/// The terminal is restored to its original state before this function returns,
/// even in the event of an error.
pub fn run_tui() -> anyhow::Result<Option<String>> {
    // ── Setup ────────────────────────────────────────────────────────────────
    enable_raw_mode().map_err(AppError::terminal_io)?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(AppError::terminal_io)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(AppError::terminal_io)?;

    let mut app = App::new();
    let result = run_loop(&mut terminal, &mut app);

    // ── Teardown (always executed) ────────────────────────────────────────────
    disable_raw_mode().map_err(AppError::terminal_io)?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(AppError::terminal_io)?;

    terminal.show_cursor().map_err(AppError::terminal_io)?;

    // Propagate any error that occurred inside the loop only after the
    // terminal has been cleanly restored.
    result?;

    match app.status {
        AppStatus::Confirmed => Ok(Some(app.buffer_text())),
        AppStatus::Cancelled | AppStatus::Running => Ok(None),
    }
}

/// Inner event loop.  Separated from `run_tui` so that teardown is guaranteed.
fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if !event::handle_event(app)? {
            break;
        }
    }
    Ok(())
}
