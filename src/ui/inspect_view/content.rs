use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use time::{OffsetDateTime, UtcOffset, macros::format_description};

use crate::model::{CommitDetails, CommitRow, CommitSignature, RefDecoration, RefKind};

use super::{
    diffstat::{diffstat_file_lines, diffstat_summary_line},
    patch::patch_line,
};

pub(super) fn details_lines(details: &CommitDetails) -> Vec<Line<'static>> {
    let mut lines = details_header_lines(details);
    lines.push(Line::from(""));

    for line in details.message.lines() {
        lines.push(Line::from(format!("    {line}")));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("---"));
    lines.extend(diffstat_file_lines(details));
    lines.push(diffstat_summary_line(details));

    if !details.patch.lines.is_empty() {
        lines.push(Line::from(""));
        lines.extend(details.patch.lines.iter().map(patch_line));
    }

    lines
}

fn details_header_lines(details: &CommitDetails) -> Vec<Line<'static>> {
    let row = &details.row;
    let mut lines = vec![
        commit_line(row),
        signature_line("Author", &details.author, Style::default().fg(Color::Blue)),
        date_line("AuthorDate", &details.author),
        signature_line(
            "Commit",
            &details.committer,
            Style::default().fg(Color::Magenta),
        ),
        date_line("CommitDate", &details.committer),
    ];

    append_optional_refs(row, &mut lines);
    lines
}

pub(super) fn row_lines(row: &CommitRow) -> Vec<Line<'static>> {
    let mut lines = vec![
        commit_line(row),
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
    ];

    append_optional_refs(row, &mut lines);
    lines
}

fn append_optional_refs(row: &CommitRow, lines: &mut Vec<Line<'static>>) {
    if !row.parent_ids.is_empty() {
        lines.push(field_line(
            "Parents",
            parents_text(row),
            Style::default().fg(Color::Yellow),
        ));
    }
    if !row.refs.is_empty() {
        lines.push(refs_line(row));
    }
}

fn commit_line(row: &CommitRow) -> Line<'static> {
    Line::styled(
        format!("commit {}", row.id),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
}

fn field_line(label: &'static str, value: String, value_style: Style) -> Line<'static> {
    Line::from(vec![label_span(label), Span::styled(value, value_style)])
}

fn label_span(label: &'static str) -> Span<'static> {
    const LABEL_WIDTH: usize = 11;

    Span::styled(
        format!("{:<LABEL_WIDTH$} ", format!("{label}:")),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
}

fn signature_line(
    label: &'static str,
    signature: &CommitSignature,
    value_style: Style,
) -> Line<'static> {
    field_line(
        label,
        format!("{} <{}>", signature.name, signature.email),
        value_style,
    )
}

fn date_line(label: &'static str, signature: &CommitSignature) -> Line<'static> {
    field_line(
        label,
        format_signature_time(signature),
        Style::default().fg(Color::Yellow),
    )
}

fn refs_line(row: &CommitRow) -> Line<'static> {
    let refs = visible_refs(row);
    let mut spans = vec![label_span("Refs")];

    for (index, reference) in refs.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(", "));
        }
        let (label, color) = match reference.kind {
            RefKind::Branch => (format!("[{}]", reference.name), Color::Green),
            RefKind::Remote => (format!("{{{}}}", reference.name), Color::Red),
            RefKind::Tag => (format!("tag:{}", reference.name), Color::Yellow),
            RefKind::Head => ("HEAD".to_string(), Color::Cyan),
        };
        spans.push(Span::styled(label, Style::default().fg(color)));
    }
    Line::from(spans)
}

fn visible_refs(row: &CommitRow) -> Vec<&RefDecoration> {
    let non_head = row
        .refs
        .iter()
        .filter(|reference| reference.kind != RefKind::Head)
        .collect::<Vec<_>>();
    if non_head.is_empty() {
        row.refs.iter().collect()
    } else {
        non_head
    }
}

fn parents_text(row: &CommitRow) -> String {
    row.parent_ids
        .iter()
        .map(|id| id.short())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn format_signature_time(signature: &CommitSignature) -> String {
    const FMT: &[time::format_description::BorrowedFormatItem<'_>] = format_description!(
        "[weekday repr:short] [month repr:short] [day padding:none] [hour]:[minute]:[second] [year] [offset_hour sign:mandatory][offset_minute]"
    );

    let Ok(utc) = OffsetDateTime::from_unix_timestamp(signature.time) else {
        return "??? ??? ? ??:??:?? ???? +0000".to_string();
    };
    let offset = UtcOffset::from_whole_seconds(signature.offset_seconds).unwrap_or(UtcOffset::UTC);

    utc.to_offset(offset)
        .format(&FMT)
        .unwrap_or_else(|_| "??? ??? ? ??:??:?? ???? +0000".to_string())
}
