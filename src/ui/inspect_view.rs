use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    app::App,
    model::{CommitDetails, CommitId, CommitRow, RefKind},
    state::{AppState, ViewMode},
};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let id = match &app.state.view {
        ViewMode::Inspect(id) => id,
        ViewMode::Log => return,
    };
    let title = format!(" Commit {} ", id.short());
    let paragraph = Paragraph::new(build_lines(&app.state, id))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
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

fn details_lines(details: &CommitDetails) -> Vec<Line<'static>> {
    let mut lines = row_lines(&details.row);
    lines.push(Line::from(""));
    lines.push(Line::from(format!(
        "{} files changed, {} insertions(+), {} deletions(-)",
        details.diffstat.files_changed, details.diffstat.insertions, details.diffstat.deletions
    )));
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "Message",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));

    for line in details.message.lines() {
        lines.push(Line::from(line.to_string()));
    }

    lines
}

fn row_lines(row: &CommitRow) -> Vec<Line<'static>> {
    vec![
        field_line("Commit", row.id.to_string()),
        field_line("Author", row.author.clone()),
        field_line("Date", row.formatted_time()),
        field_line("Parents", parents_text(row)),
        field_line("Refs", refs_text(row)),
        field_line("Summary", row.summary.clone()),
    ]
}

fn field_line(label: &'static str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<8}"),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}

fn parents_text(row: &CommitRow) -> String {
    if row.parent_ids.is_empty() {
        return "(none)".to_string();
    }

    row.parent_ids
        .iter()
        .map(|id| id.short())
        .collect::<Vec<_>>()
        .join(" ")
}

fn refs_text(row: &CommitRow) -> String {
    if row.refs.is_empty() {
        return "(none)".to_string();
    }

    row.refs
        .iter()
        .map(|reference| match reference.kind {
            RefKind::Branch | RefKind::Remote => reference.name.clone(),
            RefKind::Tag => format!("tag:{}", reference.name),
            RefKind::Head => "HEAD".to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
