/// Represents a single git commit with the information needed to display.
#[expect(dead_code)]
pub struct CommitInfo {
    /// Full commit hash (hex).
    pub id: git2::Oid,
    /// Commit summary (first line of message).
    pub summary: String,
    /// Author name.
    pub author: String,
    /// Commit time as a Unix timestamp.
    pub time: i64,
}

impl CommitInfo {
    /// Abbreviated hash (first 7 chars).
    pub fn short_hash(&self) -> String {
        self.id.to_string()[..7].to_string()
    }
}
