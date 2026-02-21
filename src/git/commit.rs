use chrono::{DateTime, Local, Utc};

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
    /// Abbreviated hash (first 7 chars).
    pub fn short_hash(&self) -> String {
        self.id.to_string()[..7].to_string()
    }

    /// Format the commit time as "YYYY-MM-DD HH:MM".
    pub fn formatted_time(&self) -> String {
        DateTime::<Utc>::from_timestamp(self.time, 0)
            .unwrap_or_default()
            .with_timezone(&Local)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    }
}
