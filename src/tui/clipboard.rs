//! Clipboard integration via the [`arboard`] crate.
//!
//! Wraps the platform clipboard with a simple, ergonomic interface and maps
//! `arboard` errors to [`crate::error::AppError::Clipboard`] so callers do
//! not need to depend on `arboard` directly.

use arboard::Clipboard;

use crate::error::AppError;

/// Read the current text content from the system clipboard.
///
/// Returns an empty string if the clipboard contains no text data.
/// Returns [`AppError::Clipboard`] if the clipboard cannot be accessed.
pub fn read_clipboard() -> Result<String, AppError> {
    let mut ctx = Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;
    ctx.get_text()
        .map_err(|e| AppError::Clipboard(e.to_string()))
}
