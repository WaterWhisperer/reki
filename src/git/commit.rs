use std::fmt;

use time::{OffsetDateTime, UtcOffset, macros::format_description};

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
