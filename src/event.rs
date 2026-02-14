use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

/// Handles terminal input events.
pub struct EventHandler;

impl EventHandler {
    /// Poll for a key press event with a 50ms timeout.
    /// Returns `Some(KeyEvent)` if a key was pressed, `None` on timeout.
    pub fn poll() -> Result<Option<KeyEvent>> {
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            // Only handle key press events (ignore release/repeat on some terminals).
            if key.kind == KeyEventKind::Press {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }
}
