use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
};

use crate::model::{PatchLine, PatchLineKind};

pub(super) fn patch_line(line: &PatchLine) -> Line<'static> {
    let text = expand_tabs(&line.text);
    Line::styled(text, patch_line_style(line.kind, &line.text))
}

fn expand_tabs(text: &str) -> String {
    const TAB_WIDTH: usize = 8;

    if !text.contains('\t') {
        return text.to_string();
    }

    let mut expanded = String::with_capacity(text.len());
    let mut column = 0usize;
    for character in text.chars() {
        if character == '\t' {
            let spaces = TAB_WIDTH - (column % TAB_WIDTH);
            expanded.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else {
            expanded.push(character);
            column += 1;
        }
    }
    expanded
}

fn patch_line_style(kind: PatchLineKind, text: &str) -> Style {
    match kind {
        PatchLineKind::FileHeader => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        PatchLineKind::HunkHeader => Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
        PatchLineKind::Context => Style::default().fg(Color::Reset),
        PatchLineKind::Addition => Style::default().fg(Color::Green),
        PatchLineKind::Deletion => Style::default().fg(Color::Red),
        PatchLineKind::Meta if text.starts_with("--- ") || text.starts_with("+++ ") => {
            Style::default().fg(Color::Yellow)
        }
        PatchLineKind::Meta => Style::default().fg(Color::Blue),
    }
}
