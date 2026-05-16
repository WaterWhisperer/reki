use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::model::CommitDetails;

pub(super) const MAX_DIFFSTAT_FILE_LINES: usize = 200;

pub(super) fn diffstat_file_lines(details: &CommitDetails) -> Vec<Line<'static>> {
    let visible_count = details.diffstat.files.len().min(MAX_DIFFSTAT_FILE_LINES);
    let visible_files = &details.diffstat.files[..visible_count];
    let path_width = visible_files
        .iter()
        .map(|file| file.path.chars().count())
        .max()
        .unwrap_or(0);
    let max_changed = visible_files
        .iter()
        .map(|file| file.insertions + file.deletions)
        .max()
        .unwrap_or(0);

    let mut lines = visible_files
        .iter()
        .map(|file| {
            let path_len = file.path.chars().count();
            let changed = file.insertions + file.deletions;
            let graph = diffstat_graph(file.insertions, file.deletions, max_changed);

            Line::from(vec![
                Span::raw(format!(" {}", file.path)),
                Span::styled(
                    format!(
                        "{:padding$} | ",
                        "",
                        padding = path_width.saturating_sub(path_len)
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{changed:>4} "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(graph.additions, Style::default().fg(Color::Green)),
                Span::styled(graph.deletions, Style::default().fg(Color::Red)),
            ])
        })
        .collect::<Vec<_>>();

    let omitted = details.diffstat.files_changed.saturating_sub(visible_count);
    if omitted > 0 {
        let word = plural(omitted, "file", "files");
        lines.push(Line::styled(
            format!(" ... {omitted} more {word}"),
            Style::default().fg(Color::DarkGray),
        ));
    }

    lines
}

pub(super) fn diffstat_summary_line(details: &CommitDetails) -> Line<'static> {
    let file_word = plural(details.diffstat.files_changed, "file", "files");
    let insertion_word = plural(details.diffstat.insertions, "insertion", "insertions");
    let deletion_word = plural(details.diffstat.deletions, "deletion", "deletions");

    Line::from(vec![
        Span::styled(
            format!(" {} {file_word} changed, ", details.diffstat.files_changed),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("{} {insertion_word}(+)", details.diffstat.insertions),
            Style::default().fg(Color::Green),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} {deletion_word}(-)", details.diffstat.deletions),
            Style::default().fg(Color::Red),
        ),
    ])
}

fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}

struct DiffStatGraph {
    additions: String,
    deletions: String,
}

fn diffstat_graph(insertions: usize, deletions: usize, max_changed: usize) -> DiffStatGraph {
    const MAX_WIDTH: usize = 49;

    let total = insertions + deletions;
    if total == 0 || max_changed == 0 {
        return DiffStatGraph {
            additions: String::new(),
            deletions: String::new(),
        };
    }

    let sides = usize::from(insertions > 0) + usize::from(deletions > 0);
    let width = if max_changed <= MAX_WIDTH {
        total
    } else {
        scaled_stat_total(total, max_changed, MAX_WIDTH)
    }
    .max(sides)
    .min(MAX_WIDTH);

    let mut addition_width = scaled_stat_width(insertions, total, width);
    let mut deletion_width = scaled_stat_width(deletions, total, width);

    while addition_width + deletion_width > width {
        if should_trim_additions(addition_width, deletion_width, insertions, deletions) {
            addition_width -= 1;
        } else if deletion_width > 1 {
            deletion_width -= 1;
        } else if addition_width > 1 {
            addition_width -= 1;
        } else {
            break;
        }
    }

    DiffStatGraph {
        additions: "+".repeat(addition_width),
        deletions: "-".repeat(deletion_width),
    }
}

fn scaled_stat_total(value: usize, max: usize, width: usize) -> usize {
    value
        .saturating_mul(width)
        .saturating_add(max / 2)
        .saturating_div(max)
        .max(1)
}

fn scaled_stat_width(value: usize, total: usize, width: usize) -> usize {
    if value == 0 {
        return 0;
    }

    value.saturating_mul(width).div_ceil(total).max(1)
}

fn should_trim_additions(
    addition_width: usize,
    deletion_width: usize,
    insertions: usize,
    deletions: usize,
) -> bool {
    addition_width > 1
        && (addition_width > deletion_width
            || (addition_width == deletion_width && insertions >= deletions))
}
