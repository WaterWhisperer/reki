use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

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

            // Hash.
            spans.push(Span::styled(
                c.short_hash(),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::raw(" "));

            // Date.
            spans.push(Span::styled(
                format!("{:<w$}", c.formatted_time(), w = DATE_WIDTH),
                Style::default().fg(Color::Green),
            ));
            spans.push(Span::raw(" "));

            // Author (truncated + padded).
            let author_display = truncate_str(&c.author, AUTHOR_MAX_WIDTH);
            spans.push(Span::styled(
                format!("{:<w$}", author_display, w = AUTHOR_MAX_WIDTH),
                Style::default().fg(Color::Blue),
            ));
            spans.push(Span::raw(" "));

            // Ref decorations.
            for r in &c.refs {
                let (color, prefix, suffix) = match r.kind {
                    RefKind::Head => (Color::Cyan, "HEAD", ""),
                    RefKind::Branch => (Color::Green, "", ""),
                    RefKind::Remote => (Color::Red, "", ""),
                    RefKind::Tag => (Color::Yellow, "🏷 ", ""),
                };
                let _ = (prefix, suffix); // suppress unused for tag prefix used below
                let label = match r.kind {
                    RefKind::Head => "HEAD".to_string(),
                    RefKind::Tag => format!("🏷 {}", r.name),
                    _ => r.name.clone(),
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
        .highlight_symbol("▸ ");

    let mut state = ListState::default();
    state.select(Some(app.selected));

    frame.render_stateful_widget(list, area, &mut state);
}

/// Truncate a string to at most `max_width` characters, appending "…" if truncated.
fn truncate_str(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_width - 1).collect();
        format!("{truncated}…")
    }
}
