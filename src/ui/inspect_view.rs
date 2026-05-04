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
    state::{Action, AppState, ViewMode},
};

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let id = match &app.state.view {
        ViewMode::Inspect(id) => id,
        ViewMode::Log => return,
    };
    let title = format!(" Commit {} ", id.short());
    let lines = build_lines(&app.state, id);
    let max_scroll_y = inspect_max_scroll_y(&lines, area);
    app.state.apply(Action::SetInspectMaxScrollY(max_scroll_y));

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
    lines.push(diffstat_line(details));
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
        field_line(
            "Commit",
            row.id.to_string(),
            Style::default().fg(Color::Yellow),
        ),
        field_line(
            "Author",
            row.author.clone(),
            Style::default().fg(Color::Blue),
        ),
        field_line(
            "Date",
            row.formatted_time(),
            Style::default().fg(Color::Green),
        ),
        field_line(
            "Parents",
            parents_text(row),
            Style::default().fg(Color::Yellow),
        ),
        refs_line(row),
        field_line(
            "Summary",
            row.summary.clone(),
            Style::default().fg(Color::Reset),
        ),
    ]
}

fn field_line(label: &'static str, value: String, value_style: Style) -> Line<'static> {
    Line::from(vec![label_span(label), Span::styled(value, value_style)])
}

fn label_span(label: &'static str) -> Span<'static> {
    Span::styled(
        format!("{label:<8}"),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
}

fn refs_line(row: &CommitRow) -> Line<'static> {
    let mut spans = vec![label_span("Refs")];
    if row.refs.is_empty() {
        spans.push(Span::styled("(none)", Style::default().fg(Color::DarkGray)));
        return Line::from(spans);
    }

    for (index, reference) in row.refs.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(", "));
        }
        let (label, color) = match reference.kind {
            RefKind::Branch => (reference.name.clone(), Color::Green),
            RefKind::Remote => (reference.name.clone(), Color::Red),
            RefKind::Tag => (format!("tag:{}", reference.name), Color::Yellow),
            RefKind::Head => ("HEAD".to_string(), Color::Cyan),
        };
        spans.push(Span::styled(label, Style::default().fg(color)));
    }
    Line::from(spans)
}

fn diffstat_line(details: &CommitDetails) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{} files changed, ", details.diffstat.files_changed),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("{} insertions(+)", details.diffstat.insertions),
            Style::default().fg(Color::Green),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} deletions(-)", details.diffstat.deletions),
            Style::default().fg(Color::Red),
        ),
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

fn inspect_max_scroll_y(lines: &[Line<'_>], area: Rect) -> usize {
    let visible_height = (area.height as usize).saturating_sub(2);
    let content_width = (area.width as usize).saturating_sub(2);

    wrapped_content_height(lines, content_width).saturating_sub(visible_height)
}

fn wrapped_content_height(lines: &[Line<'_>], width: usize) -> usize {
    let Ok(width) = u16::try_from(width) else {
        return usize::from(u16::MAX);
    };

    Paragraph::new(lines.to_vec())
        .wrap(Wrap { trim: false })
        .line_count(width)
}

#[cfg(test)]
mod tests {
    use ratatui::{layout::Rect, style::Color, text::Line};

    use super::{inspect_max_scroll_y, row_lines};
    use crate::model::{CommitId, CommitRow, RefDecoration, RefKind};

    fn row() -> CommitRow {
        CommitRow {
            id: CommitId::new("1111111111111111111111111111111111111111"),
            parent_ids: Vec::new(),
            graph: String::new(),
            summary: "fix search".to_string(),
            author: "A. User".to_string(),
            time: 0,
            refs: vec![
                RefDecoration {
                    name: "main".to_string(),
                    kind: RefKind::Branch,
                },
                RefDecoration {
                    name: "v0.1.0".to_string(),
                    kind: RefKind::Tag,
                },
            ],
        }
    }

    #[test]
    fn max_scroll_counts_wrapped_screen_lines() {
        let lines = vec![Line::from("abcdef"), Line::from("")];
        let area = Rect::new(0, 0, 5, 4);

        assert_eq!(inspect_max_scroll_y(&lines, area), 1);
    }

    #[test]
    fn max_scroll_counts_word_boundary_wraps() {
        let lines = vec![Line::from("aa bbb cc")];
        let area = Rect::new(0, 0, 7, 4);

        assert_eq!(inspect_max_scroll_y(&lines, area), 1);
    }

    #[test]
    fn row_lines_style_commit_and_refs() {
        let lines = row_lines(&row());

        assert_eq!(lines[0].spans[1].style.fg, Some(Color::Yellow));
        assert!(
            lines[4]
                .spans
                .iter()
                .any(|span| span.content == "main" && span.style.fg == Some(Color::Green))
        );
        assert!(
            lines[4]
                .spans
                .iter()
                .any(|span| span.content == "tag:v0.1.0" && span.style.fg == Some(Color::Yellow))
        );
    }
}
