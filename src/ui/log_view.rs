use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::App;
use crate::git::{CommitInfo, RefKind};

/// Fixed column widths for alignment.
const DATE_WIDTH: usize = 16; // "YYYY-MM-DD HH:MM"
const AUTHOR_MAX_WIDTH: usize = 16;

/// Render the log view into the given area.
pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let graph_max_width = app.graph_lines.iter().map(String::len).max().unwrap_or(0);

    let rows: Vec<Vec<Span<'static>>> = app
        .commits
        .iter()
        .enumerate()
        .map(|(idx, c)| {
            let graph_str = app.graph_lines.get(idx).map_or("", String::as_str);
            build_commit_line(c, graph_str, graph_max_width)
        })
        .collect();

    // Clamp horizontal scroll to content bounds.
    let viewport_width = (area.width as usize).saturating_sub(4); // borders + highlight symbol
    let max_content_width = rows
        .iter()
        .map(|spans| spans.iter().map(|s| s.content.width()).sum::<usize>())
        .max()
        .unwrap_or(0);
    app.max_scroll_x = max_content_width.saturating_sub(viewport_width);
    app.scroll_x = app.scroll_x.min(app.max_scroll_x);

    let items: Vec<ListItem> = rows
        .into_iter()
        .map(|spans| {
            let clipped = scroll_spans(spans, app.scroll_x);
            ListItem::new(Line::from(clipped))
        })
        .collect();

    let title = format!(" Log ({}) ", app.commits.len());

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

/// Build styled spans for a single commit row.
fn build_commit_line(
    c: &CommitInfo,
    graph_str: &str,
    graph_max_width: usize,
) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(12);

    // Graph.
    for ch in graph_str.chars() {
        let style = match ch {
            '*' => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            '|' => Style::default().fg(Color::DarkGray),
            _ => Style::default(),
        };
        spans.push(Span::styled(String::from(ch), style));
    }
    let pad = graph_max_width.saturating_sub(graph_str.len());
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }

    // Hash.
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

    // Author (truncated, padded).
    let (truncated, truncated_width) = c.author.unicode_truncate(AUTHOR_MAX_WIDTH);
    let padding = AUTHOR_MAX_WIDTH - truncated_width;
    let author_display = if truncated.len() < c.author.len() {
        format!(
            "{truncated}\u{2026}{:>w$}",
            "",
            w = padding.saturating_sub(1)
        )
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
        c.summary.clone(),
        Style::default().fg(Color::Reset),
    ));

    spans
}

/// Skip the first `offset` display columns from spans.
fn scroll_spans(spans: Vec<Span<'static>>, offset: usize) -> Vec<Span<'static>> {
    if offset == 0 {
        return spans;
    }
    let mut result = Vec::new();
    let mut col = 0;
    for span in spans {
        let style = span.style;
        let mut buf = String::new();
        for ch in span.content.chars() {
            let w = ch.width().unwrap_or(0);
            if col >= offset {
                buf.push(ch);
            }
            col += w;
        }
        if !buf.is_empty() {
            result.push(Span::styled(buf, style));
        }
    }
    result
}
