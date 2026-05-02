use crate::model::{CommitId, CommitRow};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadStatus {
    Idle,
    Loading,
    Complete,
    Failed(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ViewMode {
    Log,
    Inspect(CommitId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    StartLoading,
    CommitBatchLoaded {
        rows: Vec<CommitRow>,
        all_loaded: bool,
    },
    LoadFailed(String),
    MoveDown(usize),
    MoveUp(usize),
    OpenInspect,
    CloseInspect,
    Resize {
        width: usize,
        height: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    LoadNextBatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppState {
    pub rows: Vec<CommitRow>,
    pub selected: usize,
    pub load_status: LoadStatus,
    pub view: ViewMode,
    pub viewport_width: usize,
    pub viewport_height: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            selected: 0,
            load_status: LoadStatus::Idle,
            view: ViewMode::Log,
            viewport_width: 0,
            viewport_height: 0,
        }
    }
}

impl AppState {
    pub fn apply(&mut self, action: Action) -> Vec<Effect> {
        match action {
            Action::StartLoading => {
                if self.load_status == LoadStatus::Complete {
                    Vec::new()
                } else {
                    self.load_status = LoadStatus::Loading;
                    vec![Effect::LoadNextBatch]
                }
            }
            Action::CommitBatchLoaded { rows, all_loaded } => {
                self.rows.extend(rows);
                self.load_status = if all_loaded {
                    LoadStatus::Complete
                } else {
                    LoadStatus::Idle
                };
                self.clamp_selection();
                Vec::new()
            }
            Action::LoadFailed(message) => {
                self.load_status = LoadStatus::Failed(message);
                Vec::new()
            }
            Action::MoveDown(amount) => {
                let max = self.rows.len().saturating_sub(1);
                self.selected = self.selected.saturating_add(amount).min(max);
                Vec::new()
            }
            Action::MoveUp(amount) => {
                self.selected = self.selected.saturating_sub(amount);
                Vec::new()
            }
            Action::OpenInspect => {
                if let Some(row) = self.rows.get(self.selected) {
                    self.view = ViewMode::Inspect(row.id.clone());
                }
                Vec::new()
            }
            Action::CloseInspect => {
                self.view = ViewMode::Log;
                Vec::new()
            }
            Action::Resize { width, height } => {
                self.viewport_width = width;
                self.viewport_height = height;
                Vec::new()
            }
        }
    }

    fn clamp_selection(&mut self) {
        if self.rows.is_empty() {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(self.rows.len() - 1);
        }
    }
}
