use std::{
    path::Path,
    sync::mpsc::{Receiver, TryRecvError},
};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::git::Repo;
use crate::state::{Action, AppState, LoadStatus};
use crate::worker::{WorkerMessage, spawn_loader};

/// App state management.
pub struct App {
    /// Pure application state used by input handling and rendering.
    pub state: AppState,
    receiver: Receiver<WorkerMessage>,
}

impl App {
    /// Create a new App by opening the repo at the current directory.
    pub fn new() -> Result<Self> {
        Self::new_at(&std::env::current_dir()?)
    }

    /// Create a new App by discovering a repo from the provided path.
    pub fn new_at(path: &Path) -> Result<Self> {
        let repo = Repo::open(path)?;
        let mut app = Self {
            state: AppState::default(),
            receiver: spawn_loader(repo),
        };
        app.state.apply(Action::StartLoading);
        Ok(app)
    }

    pub fn tick(&mut self) {
        loop {
            match self.receiver.try_recv() {
                Ok(message) => {
                    self.state.apply(message.into());
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if !matches!(
                        self.state.load_status,
                        LoadStatus::Complete | LoadStatus::Failed(_)
                    ) {
                        self.state
                            .apply(Action::LoadFailed("loader stopped".to_string()));
                    }
                    break;
                }
            }
        }
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
            KeyCode::Char('G') | KeyCode::End => {
                self.state.apply(Action::JumpEnd);
            }
            KeyCode::Enter => {
                self.state.apply(Action::OpenInspect);
            }

            _ => {}
        }
    }

    fn move_down(&mut self, n: usize) {
        self.state.apply(Action::MoveDown(n));
    }
}
