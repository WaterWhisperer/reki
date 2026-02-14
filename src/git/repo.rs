use anyhow::Result;
use git2::Sort;

use super::CommitInfo;

/// Default batch size for incremental commit loading.
const BATCH_SIZE: usize = 200;

/// Wrapper around a git2 repository.
pub struct Repo {
    inner: git2::Repository,
}

impl Repo {
    /// Open a git repository at the given path (or discover from it).
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let inner = git2::Repository::discover(path)?;
        Ok(Self { inner })
    }

    /// Load commits starting from HEAD.
    /// `skip` — number of commits already loaded (to skip).
    /// Returns up to `BATCH_SIZE` commits.
    pub fn load_commits(&self, skip: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.inner.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let commits: Vec<CommitInfo> = revwalk
            .skip(skip)
            .take(BATCH_SIZE)
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| {
                let commit = self.inner.find_commit(oid).ok()?;
                Some(CommitInfo {
                    id: oid,
                    summary: commit.summary().unwrap_or("").to_string(),
                    author: commit.author().name().unwrap_or("unknown").to_string(),
                    time: commit.time().seconds(),
                })
            })
            .collect();

        Ok(commits)
    }
}
