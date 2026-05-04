mod inspect_view;
mod log_view;

use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Frame, Terminal,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Style},
    text::Line,
    widgets::Paragraph,
};

use crate::app::App;
use crate::state::{Action, AppState, SearchMode, ViewMode};

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
    let (content_area, status_area) = split_status_area(area);
    app.state.apply(Action::Resize {
        width: content_area.width as usize,
        height: content_area.height as usize,
    });
    match app.state.view {
        ViewMode::Log => log_view::render(frame, app, content_area),
        ViewMode::Inspect(_) => inspect_view::render(frame, app, content_area),
    }
    if let Some(area) = status_area {
        render_status(frame, &app.state, area);
    }
}

fn split_status_area(area: Rect) -> (Rect, Option<Rect>) {
    if area.height < 2 {
        return (area, None);
    }

    let [content_area, status_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .areas(area);
    (content_area, Some(status_area))
}

fn render_status(frame: &mut Frame, state: &AppState, area: Rect) {
    let status =
        Paragraph::new(Line::from(status_text(state))).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, area);
}

fn status_text(state: &AppState) -> String {
    match state.view {
        ViewMode::Inspect(_) => "j/k:scroll  q:close".to_string(),
        ViewMode::Log => match state.search_mode {
            SearchMode::Editing => format!("/{}", state.search_query),
            SearchMode::Active if !state.search_query.is_empty() => {
                format!("/{}  n:next N:previous", state.search_query)
            }
            SearchMode::Active | SearchMode::Inactive => {
                "j/k:move  Enter:inspect  /:search  q:quit".to_string()
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::status_text;
    use crate::{
        model::{CommitId, CommitRow},
        state::{Action, AppState},
    };

    fn row(summary: &str) -> CommitRow {
        CommitRow {
            id: CommitId::new("1111111111111111111111111111111111111111"),
            parent_ids: Vec::new(),
            graph: String::new(),
            summary: summary.to_string(),
            author: "A. User".to_string(),
            time: 0,
            refs: Vec::new(),
        }
    }

    #[test]
    fn status_text_shows_search_prompt_while_editing() {
        let mut state = AppState::default();

        state.apply(Action::BeginSearch);
        state.apply(Action::PushSearchChar('f'));

        assert_eq!(status_text(&state), "/f");
    }

    #[test]
    fn status_text_keeps_active_search_visible_after_enter() {
        let mut state = AppState::default();

        state.apply(Action::BeginSearch);
        state.apply(Action::PushSearchChar('f'));
        state.apply(Action::PushSearchChar('i'));
        state.apply(Action::PushSearchChar('x'));
        state.apply(Action::FinishSearch);

        assert_eq!(status_text(&state), "/fix  n:next N:previous");
    }

    #[test]
    fn status_text_hides_active_search_controls_in_inspect() {
        let mut state = AppState::default();
        state.apply(Action::CommitBatchLoaded {
            rows: vec![row("fix search status")],
            all_loaded: true,
        });

        state.apply(Action::BeginSearch);
        state.apply(Action::PushSearchChar('f'));
        state.apply(Action::PushSearchChar('i'));
        state.apply(Action::PushSearchChar('x'));
        state.apply(Action::FinishSearch);
        state.apply(Action::OpenInspect);

        assert_eq!(status_text(&state), "j/k:scroll  q:close");
    }
}
