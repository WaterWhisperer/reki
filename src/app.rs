use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::git::{CommitInfo, Repo};

/// App state management.
pub struct App {
    /// Whether the application should quit.
    pub should_quit: bool,
    /// Git repository handle.
    repo: Repo,
    /// Loaded commit list.
    pub commits: Vec<CommitInfo>,
    /// Whether all commits have been loaded.
    pub all_loaded: bool,
}

impl App {
    /// Create a new App by opening the repo at the current directory.
    pub fn new() -> Result<Self> {
        let repo = Repo::open(&std::env::current_dir()?)?;
        let mut app = Self {
            should_quit: false,
            repo,
            commits: Vec::new(),
            all_loaded: false,
        };
        app.load_more_commits()?;
        Ok(app)
    }

    /// Load the next batch of commits.
    pub fn load_more_commits(&mut self) -> Result<()> {
        if self.all_loaded {
            return Ok(());
        }
        let batch = self.repo.load_commits(self.commits.len())?;
        if batch.is_empty() {
            self.all_loaded = true;
        } else {
            self.commits.extend(batch);
        }
        Ok(())
    }

    /// Handle a key event.
    pub fn handle_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }
}
