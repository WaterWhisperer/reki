use std::{path::Path, sync::mpsc::TryRecvError};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::git::Repo;
use crate::state::{Action, AppState, LoadStatus, SearchMode, ViewMode};
use crate::worker::{WorkerCommand, WorkerHandle, spawn_loader};

/// App state management.
pub struct App {
    /// Pure application state used by input handling and rendering.
    pub state: AppState,
    worker: WorkerHandle,
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
            worker: spawn_loader(repo),
        };
        app.state.apply(Action::StartLoading);
        Ok(app)
    }

    pub fn tick(&mut self) {
        loop {
            match self.worker.messages.try_recv() {
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
        if matches!(self.state.view, ViewMode::Inspect(_)) {
            self.handle_inspect_event(event);
            return;
        }

        if self.state.search_mode == SearchMode::Editing {
            self.handle_search_event(event);
            return;
        }

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
            KeyCode::Char('/') => {
                self.state.apply(Action::BeginSearch);
            }
            KeyCode::Char('n') => {
                self.state.apply(Action::FindNext);
            }
            KeyCode::Char('N') => {
                self.state.apply(Action::FindPrevious);
            }
            KeyCode::Enter => self.open_inspect(),

            _ => {}
        }
    }

    fn handle_inspect_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.state.apply(Action::CloseInspect);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.state.apply(Action::MoveInspectDown(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.apply(Action::MoveInspectUp(1));
            }
            KeyCode::Char(' ') | KeyCode::PageDown => {
                let page = self.state.inspect_visible_height.max(1);
                self.state.apply(Action::MoveInspectDown(page));
            }
            KeyCode::Char('-') | KeyCode::Char('a') | KeyCode::PageUp => {
                let page = self.state.inspect_visible_height.max(1);
                self.state.apply(Action::MoveInspectUp(page));
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.state.apply(Action::JumpInspectTop);
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.state.apply(Action::JumpInspectEnd);
            }
            _ => {}
        }
    }

    fn handle_search_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Esc => {
                self.state.apply(Action::CancelSearch);
            }
            KeyCode::Enter => {
                self.state.apply(Action::FinishSearch);
            }
            KeyCode::Backspace => {
                self.state.apply(Action::PopSearchChar);
            }
            KeyCode::Char(ch) => {
                self.state.apply(Action::PushSearchChar(ch));
            }
            _ => {}
        }
    }

    fn move_down(&mut self, n: usize) {
        self.state.apply(Action::MoveDown(n));
    }

    fn open_inspect(&mut self) {
        let Some(row) = self.state.rows.get(self.state.selected) else {
            return;
        };
        let id = row.id.clone();
        self.state.apply(Action::OpenInspect);
        let _ = self.worker.commands.send(WorkerCommand::LoadDetails(id));
    }
}
