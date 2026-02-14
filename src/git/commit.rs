/// Represents a single git commit with the information need to display.
#[expect(unused)]
pub struct CommitInfo {
    /// Abbreviated commit hash.
    pub short_hash: String,
    /// Commit summary (first line of message).
    pub summary: String,
    /// Author name.
    pub author: String,
    /// Commit time as a Unix timestamp.
    pub time: i64,
}
