use anyhow::Result;

#[expect(unused)]
use super::CommitInfo;

/// Wrapper around a git2 repository.
#[expect(unused)]
pub struct Repo {
    inner: git2::Repository,
}

impl Repo {
    /// Open a git repository at the given path (or discover from it).
    #[expect(unused)]
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let inner = git2::Repository::discover(path)?;
        Ok(Self { inner })
    }
}
