use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::App;
use crate::model::{CommitRow, RefKind};
use crate::state::{Action, SearchMode};

/// Fixed column widths for alignment.
const DATE_WIDTH: usize = 16; // "YYYY-MM-DD HH:MM"
const AUTHOR_MAX_WIDTH: usize = 16;

/// Render the log view into the given area.
pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let graph_max_width = app
        .state
        .rows
        .iter()
        .map(|row| row.graph.len())
        .max()
        .unwrap_or(0);

    let search_query = match app.state.search_mode {
        SearchMode::Active if !app.state.search_query.is_empty() => {
            Some(app.state.search_query.as_str())
        },
        SearchMode::Active | SearchMode::Editing | SearchMode::Inactive => None,
    };

    let rows: Vec<Vec<Span<'static>>> = app
        .state
        .rows
        .iter()
        .map(|row| build_commit_line(row, graph_max_width, search_query))
        .collect();

    // Clamp horizontal scroll to content bounds.
    let viewport_width = (area.width as usize).saturating_sub(4); // borders + highlight symbol
    let max_content_width = rows
        .iter()
        .map(|spans| spans.iter().map(|s| s.content.width()).sum::<usize>())
        .max()
        .unwrap_or(0);
    app.state.apply(Action::SetMaxScrollX(
        max_content_width.saturating_sub(viewport_width),
    ));

    let items: Vec<ListItem> = rows
        .into_iter()
        .map(|spans| {
            let clipped = scroll_spans(spans, app.state.scroll_x);
            ListItem::new(Line::from(clipped))
        })
        .collect();

    let title = format!(" Log ({}) ", app.state.rows.len());

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
    state.select(Some(app.state.selected));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Build styled spans for a single commit row.
fn build_commit_line(
    row: &CommitRow,
    graph_max_width: usize,
    search_query: Option<&str>,
) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(12);

    // Graph.
    for ch in row.graph.chars() {
        let style = match ch {
            '*' => Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            '|' => Style::default().fg(Color::DarkGray),
            _ => Style::default(),
        };
        spans.push(Span::styled(String::from(ch), style));
    }
    let pad = graph_max_width.saturating_sub(row.graph.len());
    if pad > 0 {
        spans.push(Span::raw(" ".repeat(pad)));
    }

    // Hash.
    push_searchable_text(
        &mut spans,
        row.id.short().to_string(),
        Style::default().fg(Color::Yellow),
        search_query,
    );
    spans.push(Span::raw(" "));

    // Date.
    spans.push(Span::styled(
        format!("{:<w$}", row.formatted_time(), w = DATE_WIDTH),
        Style::default().fg(Color::Green),
    ));
    spans.push(Span::raw(" "));

    // Author (truncated, padded).
    let (truncated, truncated_width) = row.author.unicode_truncate(AUTHOR_MAX_WIDTH);
    let padding = AUTHOR_MAX_WIDTH - truncated_width;
    let author_display = if truncated.len() < row.author.len() {
        format!(
            "{truncated}\u{2026}{:>w$}",
            "",
            w = padding.saturating_sub(1)
        )
    } else {
        format!("{truncated}{:>w$}", "", w = padding)
    };
    push_searchable_text(
        &mut spans,
        author_display,
        Style::default().fg(Color::Blue),
        search_query,
    );
    spans.push(Span::raw(" "));

    // Ref decorations.
    for r in &row.refs {
        let color = match r.kind {
            RefKind::Head => Color::Cyan,
            RefKind::Branch => Color::Green,
            RefKind::Remote => Color::Red,
            RefKind::Tag => Color::Yellow,
        };
        let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        spans.push(Span::styled("(", style));
        if r.kind == RefKind::Tag {
            spans.push(Span::styled("\u{1f3f7} ", style));
        }
        let label = if r.kind == RefKind::Head {
            "HEAD".to_string()
        } else {
            r.name.clone()
        };
        push_searchable_text(&mut spans, label, style, search_query);
        spans.push(Span::styled(") ", style));
    }

    // Summary.
    push_searchable_text(
        &mut spans,
        row.summary.clone(),
        Style::default().fg(Color::Reset),
        search_query,
    );

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

fn push_searchable_text(
    spans: &mut Vec<Span<'static>>,
    text: String,
    style: Style,
    query: Option<&str>,
) {
    if let Some(query) = query {
        spans.extend(highlight_searchable_text(text, style, query));
    } else {
        spans.push(Span::styled(text, style));
    }
}

fn highlight_searchable_text(text: String, style: Style, query: &str) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(text, style)];
    }

    let mut highlighted = Vec::new();
    let ranges = case_insensitive_match_ranges(&text, query);
    if ranges.is_empty() {
        highlighted.push(Span::styled(text, style));
        return highlighted;
    }

    let mut cursor = 0;
    for range in ranges {
        if cursor < range.start {
            highlighted.push(Span::styled(text[cursor..range.start].to_string(), style));
        }
        highlighted.push(Span::styled(
            text[range.clone()].to_string(),
            style.patch(search_match_style(style)),
        ));
        cursor = range.end;
    }
    if cursor < text.len() {
        highlighted.push(Span::styled(text[cursor..].to_string(), style));
    }
    highlighted
}

fn case_insensitive_match_ranges(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    let query = query.to_lowercase();
    let (lowered, original_boundaries) = lowercase_with_original_boundaries(text);

    lowered
        .match_indices(query.as_str())
        .map(|(start, matched)| {
            original_boundaries[start]..original_boundaries[start + matched.len()]
        })
        .collect()
}

fn lowercase_with_original_boundaries(text: &str) -> (String, Vec<usize>) {
    let mut lowered = String::new();
    let mut original_boundaries = vec![0];

    for (start, ch) in text.char_indices() {
        let end = start + ch.len_utf8();
        for lower in ch.to_lowercase() {
            let lower = lower.to_string();
            for _ in 0..lower.len() {
                original_boundaries.push(end);
            }
            lowered.push_str(&lower);
        }
    }

    (lowered, original_boundaries)
}

fn search_match_style(base_style: Style) -> Style {
    let background = if base_style.fg == Some(Color::Yellow) {
        Color::DarkGray
    } else {
        Color::Yellow
    };

    Style::default().bg(background).add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Color, Style};
    use ratatui::text::Span;

    use super::{build_commit_line, highlight_searchable_text};
    use crate::model::{CommitId, CommitRow};

    fn row() -> CommitRow {
        CommitRow {
            id: CommitId::new("1111111111111111111111111111111111111111"),
            parent_ids: Vec::new(),
            graph: String::new(),
            summary: "fix search".to_string(),
            author: "A. User".to_string(),
            time: 1767225600,
            refs: Vec::new(),
        }
    }

    fn is_search_highlighted(span: &Span<'_>) -> bool {
        span.style.bg == Some(Color::Yellow)
    }

    #[test]
    fn highlight_matches_marks_case_insensitive_segments() {
        let spans = highlight_searchable_text(
            "Fix bug fix".to_string(),
            Style::default().fg(Color::White),
            "fix",
        );

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].content.as_ref(), "Fix");
        assert_eq!(spans[0].style.bg, Some(Color::Yellow));
        assert_eq!(spans[2].content.as_ref(), "fix");
        assert_eq!(spans[2].style.bg, Some(Color::Yellow));
    }

    #[test]
    fn search_highlight_preserves_base_foreground() {
        let spans =
            highlight_searchable_text("Fix".to_string(), Style::default().fg(Color::White), "fix");

        assert_eq!(spans[0].style.fg, Some(Color::White));
        assert_eq!(spans[0].style.bg, Some(Color::Yellow));
    }

    #[test]
    fn search_highlight_avoids_yellow_on_yellow_fields() {
        let spans =
            highlight_searchable_text("111".to_string(), Style::default().fg(Color::Yellow), "111");

        assert_eq!(spans[0].style.fg, Some(Color::Yellow));
        assert_eq!(spans[0].style.bg, Some(Color::DarkGray));
    }

    #[test]
    fn search_highlighting_ignores_non_searchable_date_text() {
        let spans = build_commit_line(&row(), 0, Some("2026"));

        assert!(!spans.iter().any(is_search_highlighted));
    }
}
