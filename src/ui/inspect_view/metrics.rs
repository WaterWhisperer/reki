use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Paragraph, Wrap},
};

pub(super) fn highlight_visible_cursor(
    frame: &mut Frame,
    area: Rect,
    cursor_y: usize,
    scroll_y: usize,
) {
    if let Some(area) = cursor_highlight_area(area, cursor_y, scroll_y) {
        frame.buffer_mut().set_style(area, current_line_style());
    }
}

pub(super) fn cursor_highlight_area(area: Rect, cursor_y: usize, scroll_y: usize) -> Option<Rect> {
    let content = inspect_content_area(area);
    let visible_y = cursor_y.checked_sub(scroll_y)?;
    if visible_y >= usize::from(content.height) || content.width == 0 {
        return None;
    }

    Some(Rect::new(
        content.x,
        content.y + u16::try_from(visible_y).ok()?,
        content.width,
        1,
    ))
}

fn inspect_content_area(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn current_line_style() -> Style {
    Style::default().bg(Color::DarkGray)
}

#[cfg(test)]
pub(super) fn inspect_max_scroll_y(lines: &[Line<'_>], area: Rect) -> usize {
    let (line_count, visible_height) = inspect_metrics(lines, area);

    line_count.saturating_sub(visible_height)
}

pub(super) fn inspect_metrics(lines: &[Line<'_>], area: Rect) -> (usize, usize) {
    let visible_height = (area.height as usize).saturating_sub(2);
    let content_width = (area.width as usize).saturating_sub(2);

    (wrapped_content_height(lines, content_width), visible_height)
}

fn wrapped_content_height(lines: &[Line<'_>], width: usize) -> usize {
    let Ok(width) = u16::try_from(width) else {
        return usize::from(u16::MAX);
    };

    Paragraph::new(lines.to_vec())
        .wrap(Wrap { trim: false })
        .line_count(width)
}
