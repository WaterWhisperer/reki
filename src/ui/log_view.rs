use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use unicode_truncate::UnicodeTruncateStr;

use crate::app::App;
use crate::git::RefKind;

/// Fixed column widths for alignment.
const DATE_WIDTH: usize = 16; // "YYYY-MM-DD HH:MM"
const AUTHOR_MAX_WIDTH: usize = 16;

/// Render the log view (commit list) into the given area.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .commits
        .iter()
        .map(|c| {
            let mut spans = Vec::with_capacity(10);

            // Hash (7 chars).
            spans.push(Span::styled(
                format!("{:.7}", c.id),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::raw(" "));

            // Date.
            spans.push(Span::styled(
                format!("{:<w$}", c.formatted_time(), w = DATE_WIDTH),
                Style::default().fg(Color::Green),
            ));
            spans.push(Span::raw(" "));

            // Author (truncated to display width, padded).
            let (truncated, truncated_width) =
                c.author.unicode_truncate(AUTHOR_MAX_WIDTH);
            let padding = AUTHOR_MAX_WIDTH - truncated_width;
            let author_display = if truncated.len() < c.author.len() {
                format!("{truncated}\u{2026}{:>w$}", "", w = padding.saturating_sub(1))
            } else {
                format!("{truncated}{:>w$}", "", w = padding)
            };
            spans.push(Span::styled(
                author_display,
                Style::default().fg(Color::Blue),
            ));
            spans.push(Span::raw(" "));

            // Ref decorations.
            for r in &c.refs {
                let (color, label) = match r.kind {
                    RefKind::Head => (Color::Cyan, "HEAD".to_string()),
                    RefKind::Branch => (Color::Green, r.name.clone()),
                    RefKind::Remote => (Color::Red, r.name.clone()),
                    RefKind::Tag => (Color::Yellow, format!("\u{1f3f7} {}", r.name)),
                };
                spans.push(Span::styled(
                    format!("({label}) "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
            }

            // Summary.
            spans.push(Span::styled(
                &c.summary,
                Style::default().fg(Color::Reset),
            ));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let commit_count = app.commits.len();
    let title = format!(" Log ({commit_count}) ");

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{25b8} ");

    let mut state = ListState::default();
    state.select(Some(app.selected));

    frame.render_stateful_widget(list, area, &mut state);
}
