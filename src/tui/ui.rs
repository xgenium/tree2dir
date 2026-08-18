//! Ratatui widget definitions and layout rendering.
//!
//! The TUI is divided into three vertical panes:
//! 1. **Editor** — the multi-line text area where the user pastes or types
//!    an ASCII tree.
//! 2. **Preview bar** — a single line showing live file/folder counts derived
//!    from the current buffer.
//! 3. **Help bar** — a static line listing all available hotkeys.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

/// Render the entire TUI layout into the given [`Frame`].
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.size();

    // Split the terminal vertically into three sections.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Editor (grows to fill available space)
            Constraint::Length(3), // Preview bar
            Constraint::Length(3), // Help / hotkey bar
        ])
        .split(area);

    render_editor(frame, app, chunks[0]);
    render_preview(frame, app, chunks[1]);
    render_help(frame, chunks[2]);
}

// ---------------------------------------------------------------------------
// Editor pane
// ---------------------------------------------------------------------------

fn render_editor(frame: &mut Frame, app: &App, area: Rect) {
    let title = Span::styled(
        format!(" {} ", app.title),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    // Build numbered lines for display.
    let lines: Vec<Line> = app
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let gutter = Span::styled(
                format!("{:>3}│ ", i + 1),
                Style::default().fg(Color::DarkGray),
            );
            Line::from(vec![gutter, Span::raw(line.clone())])
        })
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);

    // Position the terminal hardware cursor inside the editor block.
    if area.width > 7 && area.height > 2 {
        let cursor_x_offset = app
            .lines
            .get(app.cursor_line)
            .map(|l| l.chars().take(app.cursor_col).count())
            .unwrap_or(0) as u16;

        let cursor_x = area.x + 1 + 5 + cursor_x_offset;
        let cursor_y = area.y + 1 + app.cursor_line as u16;

        if cursor_x < area.x + area.width - 1 && cursor_y < area.y + area.height - 1 {
            frame.set_cursor(cursor_x, cursor_y);
        }
    }
}

// ---------------------------------------------------------------------------
// Preview bar
// ---------------------------------------------------------------------------

fn render_preview(frame: &mut Frame, app: &App, area: Rect) {
    let preview_text = format!(
        " [Preview] Creating: {} {}, {} {} in ./",
        app.stats.dir_count,
        pluralise(app.stats.dir_count, "folder", "folders"),
        app.stats.file_count,
        pluralise(app.stats.file_count, "file", "files"),
    );

    let paragraph = Paragraph::new(preview_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(Style::default().fg(Color::Yellow));

    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Help / hotkey bar
// ---------------------------------------------------------------------------

fn render_help(frame: &mut Frame, area: Rect) {
    let spans = vec![
        hotkey("[Enter]", " Build  "),
        Span::raw(" │ "),
        hotkey("[Ctrl+V]", " Paste  "),
        Span::raw(" │ "),
        hotkey("[Ctrl+E]", " Example"),
        Span::raw(" │ "),
        hotkey("[Esc]", " Quit   "),
    ];

    let paragraph = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hotkey<'a>(key: &'a str, label: &'a str) -> Span<'a> {
    Span::styled(
        format!("{key}{label}"),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
}

fn pluralise<'a>(n: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if n == 1 {
        singular
    } else {
        plural
    }
}
