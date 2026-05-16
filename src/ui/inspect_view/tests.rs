use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::text::Line;

use super::content::{details_lines, format_signature_time, row_lines};
use super::diffstat::{MAX_DIFFSTAT_FILE_LINES, diffstat_file_lines};
use super::metrics::{cursor_highlight_area, inspect_max_scroll_y};
use super::patch::patch_line;
use crate::model::{
    CommitDetails, CommitId, CommitRow, CommitSignature, DiffStat, DiffStatFile, Patch, PatchLine,
    PatchLineKind, RefDecoration, RefKind,
};

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
                name: "origin/main".to_string(),
                kind: RefKind::Remote,
            },
            RefDecoration {
                name: "v0.1.0".to_string(),
                kind: RefKind::Tag,
            },
        ],
    }
}

fn signature() -> CommitSignature {
    CommitSignature {
        name: "A. User".to_string(),
        email: "a@example.com".to_string(),
        time: 1_777_919_337,
        offset_seconds: 8 * 60 * 60,
    }
}

fn has_styled_line(lines: &[Line<'_>], text: &str, color: Color) -> bool {
    lines.iter().any(|line| {
        line.spans
            .first()
            .is_some_and(|span| span.content == text && line.style.fg == Some(color))
    })
}

fn assert_line_text(lines: &[Line<'_>], expected: &str) {
    assert!(
        lines.iter().any(|line| line_text(line) == expected),
        "missing line: {expected}"
    );
}

fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>()
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

    assert_eq!(lines[0].style.fg, Some(Color::Yellow));
    assert!(
        lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.content == "[main]" && span.style.fg == Some(Color::Green))
    );
    assert!(
        lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.content == "{origin/main}" && span.style.fg == Some(Color::Red))
    );
    assert!(
        lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .any(|span| span.content == "tag:v0.1.0" && span.style.fg == Some(Color::Yellow))
    );
}

#[test]
fn details_lines_match_git_show_shape() {
    let details = CommitDetails {
        row: row(),
        author: signature(),
        committer: CommitSignature {
            name: "C. User".to_string(),
            email: "c@example.com".to_string(),
            ..signature()
        },
        message: "fix search\n\nbody".to_string(),
        diffstat: DiffStat {
            files: vec![DiffStatFile {
                path: "src/model.rs".to_string(),
                insertions: 1,
                deletions: 1,
            }],
            files_changed: 1,
            insertions: 1,
            deletions: 1,
        },
        patch: Patch {
            lines: vec![
                PatchLine {
                    kind: PatchLineKind::FileHeader,
                    text: "diff --git a/a.txt b/a.txt".to_string(),
                },
                PatchLine {
                    kind: PatchLineKind::HunkHeader,
                    text: "@@ -1,1 +1,1 @@".to_string(),
                },
                PatchLine {
                    kind: PatchLineKind::Deletion,
                    text: "-one".to_string(),
                },
                PatchLine {
                    kind: PatchLineKind::Addition,
                    text: "+two".to_string(),
                },
            ],
        },
    };

    let lines = details_lines(&details);

    assert_line_text(&lines, "commit 1111111111111111111111111111111111111111");
    assert_line_text(&lines, "Author:     A. User <a@example.com>");
    assert_line_text(&lines, "Commit:     C. User <c@example.com>");
    assert_line_text(&lines, "    fix search");
    assert_line_text(&lines, "---");
    assert!(
        lines
            .iter()
            .any(|line| line_text(line).starts_with(" src/model.rs |    2 "))
    );
    assert!(has_styled_line(&lines, "+two", Color::Green));
    assert!(has_styled_line(&lines, "-one", Color::Red));
}

#[test]
fn patch_line_expands_tabs_before_rendering() {
    let line = patch_line(&PatchLine {
        kind: PatchLineKind::Addition,
        text: "+\tcommand".to_string(),
    });

    assert_eq!(line_text(&line), "+       command");
    assert_eq!(line.style.fg, Some(Color::Green));
}

#[test]
fn diffstat_file_lines_are_bounded() {
    let files = (0..MAX_DIFFSTAT_FILE_LINES)
        .map(|index| DiffStatFile {
            path: format!("file-{index:03}.rs"),
            insertions: 1,
            deletions: 0,
        })
        .collect::<Vec<_>>();
    let files_changed = files.len() + 1;
    let details = CommitDetails {
        row: row(),
        author: signature(),
        committer: signature(),
        message: String::new(),
        diffstat: DiffStat {
            files,
            files_changed,
            insertions: files_changed,
            deletions: 0,
        },
        patch: Patch { lines: Vec::new() },
    };

    let lines = diffstat_file_lines(&details);

    assert_eq!(lines.len(), MAX_DIFFSTAT_FILE_LINES + 1);
    assert_line_text(&lines, " ... 1 more file");
    assert!(
        lines
            .iter()
            .all(|line| !line_text(line).contains("file-200.rs"))
    );
}

#[test]
fn signature_time_uses_git_show_date_format() {
    assert_eq!(
        format_signature_time(&signature()),
        "Tue May 5 02:28:57 2026 +0800"
    );
}

#[test]
fn cursor_highlight_uses_visible_screen_row() {
    let area = Rect::new(10, 20, 12, 5);

    assert_eq!(
        cursor_highlight_area(area, 4, 2),
        Some(Rect::new(11, 23, 10, 1))
    );
    assert_eq!(cursor_highlight_area(area, 1, 2), None);
    assert_eq!(cursor_highlight_area(area, 5, 2), None);
}
