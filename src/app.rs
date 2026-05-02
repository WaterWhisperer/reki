use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::git::Repo;
use crate::graph::Graph;
use crate::state::{Action, AppState, LoadStatus};

/// App state management.
pub struct App {
    /// Pure application state used by input handling and rendering.
    pub state: AppState,
    /// Git repository handle.
    repo: Repo,
    /// Lane-tracking state for the ASCII graph.
    graph: Graph,
}

impl App {
    /// Create a new App by opening the repo at the current directory.
    pub fn new() -> Result<Self> {
        Self::new_at(&std::env::current_dir()?)
    }

    /// Create a new App by discovering a repo from the provided path.
    pub fn new_at(path: &std::path::Path) -> Result<Self> {
        let repo = Repo::open(path)?;
        let mut app = Self {
            state: AppState::default(),
            repo,
            graph: Graph::default(),
        };
        app.load_more_commits()?;
        Ok(app)
    }

    /// Load the next batch of commits.
    pub fn load_more_commits(&mut self) -> Result<()> {
        if self.state.load_status == LoadStatus::Complete {
            return Ok(());
        }
        self.state.apply(Action::StartLoading);

        let batch = self.repo.load_commits()?;
        let rows = batch
            .iter()
            .map(|c| {
                let mut row = c.to_row(String::new());
                row.graph = self.graph.next_row(&row.id, &row.parent_ids).text;
                row
            })
            .collect();
        self.state.apply(Action::CommitBatchLoaded {
            rows,
            all_loaded: batch.is_empty(),
        });
        Ok(())
    }

    /// Handle a key event.
    pub fn handle_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state.apply(Action::Quit);
            }

            // Single-line movement.
            KeyCode::Char('j') | KeyCode::Down => self.move_down(1),
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.apply(Action::MoveUp(1));
            }

            // Page movement.
            KeyCode::Char(' ') | KeyCode::PageDown => self.move_down(self.state.page_height),
            KeyCode::Char('-') | KeyCode::Char('a') | KeyCode::PageUp => {
                self.state.apply(Action::MoveUp(self.state.page_height));
            }

            // Horizontal scroll.
            KeyCode::Char('h') | KeyCode::Left => {
                self.state.apply(Action::ScrollLeft(2));
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.state.apply(Action::ScrollRight(2));
            }

            // Jump to top / bottom.
            KeyCode::Char('g') | KeyCode::Home => {
                self.state.apply(Action::JumpTop);
            }
            KeyCode::Char('G') | KeyCode::End => self.jump_to_end(),
            KeyCode::Enter => {
                self.state.apply(Action::OpenInspect);
            }

            _ => {}
        }
    }

    fn move_down(&mut self, n: usize) {
        self.state.apply(Action::MoveDown(n));
        self.maybe_load_more();
    }

    /// When the cursor is within one page of the end, load more commits.
    fn maybe_load_more(&mut self) {
        if self.state.load_status != LoadStatus::Complete
            && self.state.selected + self.state.page_height >= self.state.rows.len()
        {
            let _ = self.load_more_commits();
        }
    }

    /// Jump to the very last commit, loading all remaining if needed.
    fn jump_to_end(&mut self) {
        while self.state.load_status != LoadStatus::Complete {
            if self.load_more_commits().is_err() {
                break;
            }
        }
        self.state.apply(Action::JumpEnd);
    }
}
