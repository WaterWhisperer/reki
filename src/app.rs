use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::git::{CommitInfo, Graph, Repo};

/// App state management.
pub struct App {
    /// Whether the application should quit.
    pub should_quit: bool,
    /// Git repository handle.
    repo: Repo,
    /// Loaded commit list.
    pub commits: Vec<CommitInfo>,
    /// Rendered graph line per commit (parallel to `commits`).
    pub graph_lines: Vec<String>,
    /// Lane-tracking state for the ASCII graph.
    graph: Graph,
    /// Whether all commits have been loaded.
    pub all_loaded: bool,
    /// Currently selected commit index.
    pub selected: usize,
    /// Visible rows in the log viewport (set by UI on each draw).
    pub page_height: usize,
}

impl App {
    /// Create a new App by opening the repo at the current directory.
    pub fn new() -> Result<Self> {
        let repo = Repo::open(&std::env::current_dir()?)?;
        let mut app = Self {
            should_quit: false,
            repo,
            commits: Vec::new(),
            graph_lines: Vec::new(),
            graph: Graph::new(),
            all_loaded: false,
            selected: 0,
            page_height: 20,
        };
        app.load_more_commits()?;
        Ok(app)
    }

    /// Load the next batch of commits.
    pub fn load_more_commits(&mut self) -> Result<()> {
        if self.all_loaded {
            return Ok(());
        }
        let batch = self.repo.load_commits()?;
        if batch.is_empty() {
            self.all_loaded = true;
        } else {
            for c in &batch {
                let line = self.graph.next_row(c.id, &c.parent_ids);
                self.graph_lines.push(line);
            }
            self.commits.extend(batch);
        }
        Ok(())
    }

    /// Handle a key event.
    pub fn handle_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,

            // Single-line movement.
            KeyCode::Char('j') | KeyCode::Down => self.move_down(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_up(1),

            // Page movement.
            KeyCode::Char(' ') | KeyCode::PageDown => self.move_down(self.page_height),
            KeyCode::Char('-') | KeyCode::Char('a') | KeyCode::PageUp => {
                self.move_up(self.page_height);
            }

            // Jump to top / bottom.
            KeyCode::Char('g') | KeyCode::Home => self.selected = 0,
            KeyCode::Char('G') | KeyCode::End => self.jump_to_end(),

            _ => {}
        }
    }

    fn move_down(&mut self, n: usize) {
        let max = self.commits.len().saturating_sub(1);
        self.selected = (self.selected + n).min(max);
        self.maybe_load_more();
    }

    fn move_up(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
    }

    /// When the cursor is within one page of the end, load more commits.
    fn maybe_load_more(&mut self) {
        if !self.all_loaded && self.selected + self.page_height >= self.commits.len() {
            let _ = self.load_more_commits();
        }
    }

    /// Jump to the very last commit, loading all remaining if needed.
    fn jump_to_end(&mut self) {
        while !self.all_loaded {
            if self.load_more_commits().is_err() {
                break;
            }
        }
        self.selected = self.commits.len().saturating_sub(1);
    }
}
