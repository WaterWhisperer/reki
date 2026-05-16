use std::fmt;

use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CommitId(String);

impl CommitId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn short(&self) -> &str {
        self.0.get(..7).unwrap_or(&self.0)
    }
}

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RefKind {
    Branch,
    Remote,
    Tag,
    Head,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefDecoration {
    pub name: String,
    pub kind: RefKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitRow {
    pub id: CommitId,
    pub parent_ids: Vec<CommitId>,
    pub graph: String,
    pub summary: String,
    pub author: String,
    pub time: i64,
    pub refs: Vec<RefDecoration>,
}

impl CommitRow {
    pub fn formatted_time(&self) -> String {
        const FMT: &[time::format_description::BorrowedFormatItem<'_>] =
            format_description!("[year]-[month]-[day] [hour]:[minute]");

        let Ok(utc) = OffsetDateTime::from_unix_timestamp(self.time) else {
            return String::from("????-??-?? ??:??");
        };
        let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        utc.to_offset(local_offset)
            .format(&FMT)
            .unwrap_or_else(|_| String::from("????-??-?? ??:??"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitSignature {
    pub name: String,
    pub email: String,
    pub time: i64,
    pub offset_seconds: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffStatFile {
    pub path: String,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub files: Vec<DiffStatFile>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Patch {
    pub lines: Vec<PatchLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatchLine {
    pub kind: PatchLineKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PatchLineKind {
    FileHeader,
    HunkHeader,
    Context,
    Addition,
    Deletion,
    Meta,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitDetails {
    pub row: CommitRow,
    pub author: CommitSignature,
    pub committer: CommitSignature,
    pub message: String,
    pub diffstat: DiffStat,
    pub patch: Patch,
}
