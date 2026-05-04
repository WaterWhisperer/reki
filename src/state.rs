use crate::{
    model::{CommitDetails, CommitId, CommitRow},
    search::{self, Direction},
};

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
pub enum SearchMode {
    Inactive,
    Editing,
    Active,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Quit,
    StartLoading,
    CommitBatchLoaded {
        rows: Vec<CommitRow>,
        all_loaded: bool,
    },
    CommitDetailsLoaded(CommitDetails),
    CommitDetailsFailed {
        id: CommitId,
        message: String,
    },
    LoadFailed(String),
    MoveDown(usize),
    MoveUp(usize),
    JumpTop,
    JumpEnd,
    ScrollRight(usize),
    ScrollLeft(usize),
    SetMaxScrollX(usize),
    ScrollInspectDown(usize),
    ScrollInspectUp(usize),
    SetInspectMaxScrollY(usize),
    BeginSearch,
    PushSearchChar(char),
    PopSearchChar,
    FinishSearch,
    CancelSearch,
    FindNext,
    FindPrevious,
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
    pub should_quit: bool,
    pub rows: Vec<CommitRow>,
    pub selected: usize,
    pub load_status: LoadStatus,
    pub view: ViewMode,
    pub details: Option<CommitDetails>,
    pub details_error: Option<String>,
    pub page_height: usize,
    pub scroll_x: usize,
    pub max_scroll_x: usize,
    pub viewport_width: usize,
    pub viewport_height: usize,
    pub search_mode: SearchMode,
    pub search_query: String,
    pub inspect_scroll_y: usize,
    pub inspect_max_scroll_y: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            should_quit: false,
            rows: Vec::new(),
            selected: 0,
            load_status: LoadStatus::Idle,
            view: ViewMode::Log,
            details: None,
            details_error: None,
            page_height: 20,
            scroll_x: 0,
            max_scroll_x: 0,
            viewport_width: 0,
            viewport_height: 0,
            search_mode: SearchMode::Inactive,
            search_query: String::new(),
            inspect_scroll_y: 0,
            inspect_max_scroll_y: 0,
        }
    }
}

impl AppState {
    pub fn apply(&mut self, action: Action) -> Vec<Effect> {
        match action {
            Action::Quit => {
                self.should_quit = true;
                Vec::new()
            }
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
                    LoadStatus::Loading
                };
                self.clamp_selection();
                Vec::new()
            }
            Action::CommitDetailsLoaded(details) => {
                if matches!(&self.view, ViewMode::Inspect(id) if id == &details.row.id) {
                    self.details = Some(details);
                    self.details_error = None;
                }
                Vec::new()
            }
            Action::CommitDetailsFailed { id, message } => {
                if matches!(&self.view, ViewMode::Inspect(current) if current == &id) {
                    self.details_error = Some(message);
                }
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
            Action::JumpTop => {
                self.selected = 0;
                Vec::new()
            }
            Action::JumpEnd => {
                self.selected = self.rows.len().saturating_sub(1);
                Vec::new()
            }
            Action::ScrollRight(amount) => {
                self.scroll_x = self.scroll_x.saturating_add(amount).min(self.max_scroll_x);
                Vec::new()
            }
            Action::ScrollLeft(amount) => {
                self.scroll_x = self.scroll_x.saturating_sub(amount);
                Vec::new()
            }
            Action::SetMaxScrollX(max_scroll_x) => {
                self.max_scroll_x = max_scroll_x;
                self.scroll_x = self.scroll_x.min(self.max_scroll_x);
                Vec::new()
            }
            Action::ScrollInspectDown(amount) => {
                self.inspect_scroll_y = self
                    .inspect_scroll_y
                    .saturating_add(amount)
                    .min(self.inspect_max_scroll_y);
                Vec::new()
            }
            Action::ScrollInspectUp(amount) => {
                self.inspect_scroll_y = self.inspect_scroll_y.saturating_sub(amount);
                Vec::new()
            }
            Action::SetInspectMaxScrollY(max_scroll_y) => {
                self.inspect_max_scroll_y = max_scroll_y;
                self.inspect_scroll_y = self.inspect_scroll_y.min(self.inspect_max_scroll_y);
                Vec::new()
            }
            Action::BeginSearch => {
                self.search_mode = SearchMode::Editing;
                self.search_query.clear();
                Vec::new()
            }
            Action::PushSearchChar(ch) => {
                if self.search_mode == SearchMode::Editing {
                    self.search_query.push(ch);
                }
                Vec::new()
            }
            Action::PopSearchChar => {
                if self.search_mode == SearchMode::Editing {
                    self.search_query.pop();
                }
                Vec::new()
            }
            Action::FinishSearch => {
                if self.search_query.is_empty() {
                    self.search_mode = SearchMode::Inactive;
                } else {
                    self.search_mode = SearchMode::Active;
                    self.select_search_match(Direction::Forward);
                }
                Vec::new()
            }
            Action::CancelSearch => {
                self.search_mode = SearchMode::Inactive;
                self.search_query.clear();
                Vec::new()
            }
            Action::FindNext => {
                self.select_search_match(Direction::Forward);
                Vec::new()
            }
            Action::FindPrevious => {
                self.select_search_match(Direction::Backward);
                Vec::new()
            }
            Action::OpenInspect => {
                if let Some(row) = self.rows.get(self.selected) {
                    self.view = ViewMode::Inspect(row.id.clone());
                    self.details = None;
                    self.details_error = None;
                    self.inspect_scroll_y = 0;
                    self.inspect_max_scroll_y = 0;
                }
                Vec::new()
            }
            Action::CloseInspect => {
                self.view = ViewMode::Log;
                self.details = None;
                self.details_error = None;
                self.inspect_scroll_y = 0;
                self.inspect_max_scroll_y = 0;
                Vec::new()
            }
            Action::Resize { width, height } => {
                self.viewport_width = width;
                self.viewport_height = height;
                self.page_height = height.saturating_sub(2);
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

    fn select_search_match(&mut self, direction: Direction) {
        if self.search_mode != SearchMode::Active || self.search_query.is_empty() {
            return;
        }

        let Some(index) = search::find_match(
            &self.rows,
            self.selected,
            self.search_query.as_str(),
            direction,
        ) else {
            return;
        };
        self.selected = index;
    }
}
