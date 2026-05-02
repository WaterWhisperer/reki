use std::fmt;

use time::{OffsetDateTime, UtcOffset, macros::format_description};

use crate::model::{CommitId, CommitRow, RefDecoration as RowRefDecoration, RefKind as RowRefKind};

/// Type of a git reference for display purposes.
#[derive(Clone, Debug)]
pub enum RefKind {
    /// Local branch (refs/heads/*).
    Branch,
    /// Remote-tracking branch (refs/remotes/*).
    Remote,
    /// Tag (refs/tags/*).
    Tag,
    /// HEAD (may be detached or symbolic).
    Head,
}

/// A reference decoration attached to a commit.
#[derive(Clone, Debug)]
pub struct RefDecoration {
    /// Short display name (e.g. "main", "origin/main", "v1.0").
    pub name: String,
    /// Kind of reference.
    pub kind: RefKind,
}

/// Represents a single git commit with the information needed to display.
pub struct CommitInfo {
    /// Full commit hash (hex).
    pub id: git2::Oid,
    /// Parent commit IDs.
    pub parent_ids: Vec<git2::Oid>,
    /// Commit summary (first line of message).
    pub summary: String,
    /// Author name.
    pub author: String,
    /// Commit time as a Unix timestamp.
    pub time: i64,
    /// Reference decorations (branches, tags, HEAD) pointing to this commit.
    pub refs: Vec<RefDecoration>,
}

impl CommitInfo {
    pub fn to_row(&self, graph: String) -> CommitRow {
        CommitRow {
            id: CommitId::new(self.id.to_string()),
            parent_ids: self
                .parent_ids
                .iter()
                .map(|id| CommitId::new(id.to_string()))
                .collect(),
            graph,
            summary: self.summary.clone(),
            author: self.author.clone(),
            time: self.time,
            refs: self
                .refs
                .iter()
                .map(|decoration| RowRefDecoration {
                    name: decoration.name.clone(),
                    kind: RowRefKind::from(&decoration.kind),
                })
                .collect(),
        }
    }

    /// Format the commit time as "YYYY-MM-DD HH:MM" in the local timezone.
    pub fn formatted_time(&self) -> String {
        const FMT: &[time::format_description::BorrowedFormatItem<'_>] =
            format_description!("[year]-[month]-[day] [hour]:[minute]");

        let Ok(utc) = OffsetDateTime::from_unix_timestamp(self.time) else {
            return String::from("????-??-?? ??:??");
        };
        let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let local = utc.to_offset(local_offset);
        local
            .format(&FMT)
            .unwrap_or_else(|_| String::from("????-??-?? ??:??"))
    }
}

impl fmt::Display for CommitInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.7} {}", self.id, self.summary)
    }
}

impl From<&RefKind> for RowRefKind {
    fn from(kind: &RefKind) -> Self {
        match kind {
            RefKind::Branch => Self::Branch,
            RefKind::Remote => Self::Remote,
            RefKind::Tag => Self::Tag,
            RefKind::Head => Self::Head,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CommitId;

    fn oid(byte: u8) -> git2::Oid {
        let mut bytes = [0u8; 20];
        bytes[0] = byte;
        git2::Oid::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn commit_info_converts_to_backend_neutral_row() {
        let commit = CommitInfo {
            id: oid(1),
            parent_ids: vec![oid(2), oid(3)],
            summary: "add graph".to_string(),
            author: "Ada".to_string(),
            time: 42,
            refs: vec![
                RefDecoration {
                    name: "main".to_string(),
                    kind: RefKind::Branch,
                },
                RefDecoration {
                    name: "v1.0".to_string(),
                    kind: RefKind::Tag,
                },
            ],
        };

        let row = commit.to_row("* | ".to_string());

        assert_eq!(row.id, CommitId::new(commit.id.to_string()));
        assert_eq!(
            row.parent_ids,
            vec![
                CommitId::new(oid(2).to_string()),
                CommitId::new(oid(3).to_string())
            ]
        );
        assert_eq!(row.graph, "* | ");
        assert_eq!(row.summary, "add graph");
        assert_eq!(row.author, "Ada");
        assert_eq!(row.time, 42);
        assert_eq!(row.refs[0].kind, RowRefKind::Branch);
        assert_eq!(row.refs[1].kind, RowRefKind::Tag);
    }
}
