mod content;
mod diffstat;
mod metrics;
mod patch;

#[cfg(test)]
mod tests;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use self::content::{details_lines, row_lines};
use self::metrics::{highlight_visible_cursor, inspect_metrics};
use crate::app::App;
use crate::model::CommitId;
use crate::state::{Action, AppState, ViewMode};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let id = match &app.state.view {
        ViewMode::Inspect(id) => id,
        ViewMode::Log => return,
    };
    let title = format!(" Commit {} ", id.short());
    let lines = build_lines(&app.state, id);
    let (line_count, visible_height) = inspect_metrics(&lines, area);
    app.state.apply(Action::SetInspectMetrics {
        line_count,
        visible_height,
    });

    let scroll_y = u16::try_from(app.state.inspect_scroll_y).unwrap_or(u16::MAX);
    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .scroll((scroll_y, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
    highlight_visible_cursor(
        frame,
        area,
        app.state.inspect_cursor_y,
        app.state.inspect_scroll_y,
    );
}

fn build_lines(state: &AppState, id: &CommitId) -> Vec<Line<'static>> {
    if let Some(details) = state
        .details
        .as_ref()
        .filter(|details| &details.row.id == id)
    {
        return details_lines(details);
    }

    let Some(row) = state.rows.iter().find(|row| &row.id == id) else {
        return vec![Line::from("Loading details...")];
    };

    let mut lines = row_lines(row);
    lines.push(Line::from(""));
    if let Some(message) = &state.details_error {
        lines.push(Line::styled(
            format!("Details unavailable: {message}"),
            Style::default().fg(Color::Red),
        ));
    } else {
        lines.push(Line::styled(
            "Loading details...",
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines
}
