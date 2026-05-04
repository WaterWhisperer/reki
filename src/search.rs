use crate::model::CommitRow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Direction {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MatchStart {
    IncludeSelected,
    ExcludeSelected,
}

pub(crate) fn find_match(
    rows: &[CommitRow],
    selected: usize,
    query: &str,
    direction: Direction,
    start: MatchStart,
) -> Option<usize> {
    if rows.is_empty() || query.is_empty() {
        return None;
    }

    let query = query.to_lowercase();
    let first_step = match start {
        MatchStart::IncludeSelected => 0,
        MatchStart::ExcludeSelected => 1,
    };

    for offset in 0..rows.len() {
        let step = first_step + offset;
        let index = match direction {
            Direction::Forward => (selected + step) % rows.len(),
            Direction::Backward => (selected + rows.len() - step) % rows.len(),
        };
        if row_matches_query(&rows[index], &query) {
            return Some(index);
        }
    }

    None
}

fn row_matches_query(row: &CommitRow, query: &str) -> bool {
    row.summary.to_lowercase().contains(query)
        || row.author.to_lowercase().contains(query)
        || row.id.to_string().to_lowercase().contains(query)
        || row
            .refs
            .iter()
            .any(|reference| reference.name.to_lowercase().contains(query))
}
