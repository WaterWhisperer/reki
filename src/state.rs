use crate::{
    model::{CommitDetails, CommitId, CommitRow},
    search::{self, Direction, MatchStart},
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
    CommitDetailsLoaded(Box<CommitDetails>),
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
    MoveInspectDown(usize),
    MoveInspectUp(usize),
    JumpInspectTop,
    JumpInspectEnd,
    SetInspectMetrics {
        line_count: usize,
        visible_height: usize,
    },
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
    pub inspect_cursor_y: usize,
    pub inspect_scroll_y: usize,
    pub inspect_line_count: usize,
    pub inspect_visible_height: usize,
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
            inspect_cursor_y: 0,
            inspect_scroll_y: 0,
            inspect_line_count: 0,
            inspect_visible_height: 0,
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
                    self.details = Some(*details);
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
            Action::MoveInspectDown(amount) | Action::ScrollInspectDown(amount) => {
                self.inspect_cursor_y = self
                    .inspect_cursor_y
                    .saturating_add(amount)
                    .min(self.inspect_max_cursor_y());
                self.follow_inspect_cursor();
                Vec::new()
            }
            Action::MoveInspectUp(amount) | Action::ScrollInspectUp(amount) => {
                self.inspect_cursor_y = self.inspect_cursor_y.saturating_sub(amount);
                self.follow_inspect_cursor();
                Vec::new()
            }
            Action::JumpInspectTop => {
                self.inspect_cursor_y = 0;
                self.follow_inspect_cursor();
                Vec::new()
            }
            Action::JumpInspectEnd => {
                self.inspect_cursor_y = self.inspect_max_cursor_y();
                self.follow_inspect_cursor();
                Vec::new()
            }
            Action::SetInspectMetrics {
                line_count,
                visible_height,
            } => {
                self.inspect_line_count = line_count;
                self.inspect_visible_height = visible_height;
                self.inspect_max_scroll_y = line_count.saturating_sub(visible_height);
                self.clamp_inspect();
                Vec::new()
            }
            Action::SetInspectMaxScrollY(max_scroll_y) => {
                self.inspect_max_scroll_y = max_scroll_y;
                if self.inspect_line_count == 0 && max_scroll_y > 0 {
                    self.inspect_line_count = max_scroll_y + 1;
                    self.inspect_visible_height = 1;
                }
                self.clamp_inspect();
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
                    self.select_search_match(Direction::Forward, MatchStart::IncludeSelected);
                }
                Vec::new()
            }
            Action::CancelSearch => {
                self.search_mode = SearchMode::Inactive;
                self.search_query.clear();
                Vec::new()
            }
            Action::FindNext => {
                self.select_search_match(Direction::Forward, MatchStart::ExcludeSelected);
                Vec::new()
            }
            Action::FindPrevious => {
                self.select_search_match(Direction::Backward, MatchStart::ExcludeSelected);
                Vec::new()
            }
            Action::OpenInspect => {
                if let Some(row) = self.rows.get(self.selected) {
                    self.view = ViewMode::Inspect(row.id.clone());
                    self.details = None;
                    self.details_error = None;
                    self.reset_inspect_position();
                }
                Vec::new()
            }
            Action::CloseInspect => {
                self.view = ViewMode::Log;
                self.details = None;
                self.details_error = None;
                self.reset_inspect_position();
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

    fn reset_inspect_position(&mut self) {
        self.inspect_cursor_y = 0;
        self.inspect_scroll_y = 0;
        self.inspect_line_count = 0;
        self.inspect_visible_height = 0;
        self.inspect_max_scroll_y = 0;
    }

    fn inspect_max_cursor_y(&self) -> usize {
        self.inspect_line_count.saturating_sub(1)
    }

    fn clamp_inspect(&mut self) {
        if self.inspect_line_count == 0 {
            self.inspect_cursor_y = 0;
            self.inspect_scroll_y = 0;
            self.inspect_max_scroll_y = 0;
            return;
        }

        self.inspect_max_scroll_y = self
            .inspect_max_scroll_y
            .min(self.inspect_line_count.saturating_sub(1));
        self.inspect_cursor_y = self.inspect_cursor_y.min(self.inspect_max_cursor_y());
        self.follow_inspect_cursor();
    }

    fn follow_inspect_cursor(&mut self) {
        if self.inspect_line_count == 0 {
            self.inspect_cursor_y = 0;
            self.inspect_scroll_y = 0;
            return;
        }

        let visible_height = self.inspect_visible_height.max(1);
        if self.inspect_cursor_y < self.inspect_scroll_y {
            self.inspect_scroll_y = self.inspect_cursor_y;
        } else {
            let bottom = self.inspect_scroll_y.saturating_add(visible_height - 1);
            if self.inspect_cursor_y > bottom {
                self.inspect_scroll_y = self.inspect_cursor_y + 1 - visible_height;
            }
        }
        self.inspect_scroll_y = self.inspect_scroll_y.min(self.inspect_max_scroll_y);
    }

    fn select_search_match(&mut self, direction: Direction, start: MatchStart) {
        if self.search_mode != SearchMode::Active || self.search_query.is_empty() {
            return;
        }

        let Some(index) = search::find_match(
            &self.rows,
            self.selected,
            self.search_query.as_str(),
            direction,
            start,
        ) else {
            return;
        };
        self.selected = index;
    }
}
