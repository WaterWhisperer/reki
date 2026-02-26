mod log_view;

use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Frame, Terminal, prelude::CrosstermBackend};

use crate::app::App;

type Term = Terminal<CrosstermBackend<Stdout>>;

/// Terminal UI wrapper.
pub struct Tui {
    terminal: Term,
}

impl Tui {
    /// Create a new TUI instance.
    pub fn new() -> Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Enter raw mode and alternate screen.
    pub fn enter(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Exit raw mode and alternate screen.
    pub fn exit(&mut self) -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;
        Ok(())
    }

    /// Draw the UI.
    pub fn draw(&mut self, app: &mut App) -> Result<()> {
        self.terminal.draw(|frame| {
            render(frame, app);
        })?;
        Ok(())
    }
}

/// Render the entire UI.
fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    // Update page_height: area height minus 2 for the border.
    app.page_height = (area.height as usize).saturating_sub(2);
    log_view::render(frame, app, area);
}
