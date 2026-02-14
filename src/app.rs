use crossterm::event::{KeyCode, KeyEvent};

/// App state management.
pub struct App {
    /// Whether the application should quit.
    pub should_quit: bool,
}

impl App {
    /// Create a new `App` instance.
    pub fn new() -> Self {
        Self { should_quit: false }
    }

    /// Handle a key event.
    pub fn handle_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }
}
