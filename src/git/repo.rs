use std::collections::HashMap;

use anyhow::Result;
use git2::Sort;

use super::commit::{CommitInfo, RefDecoration, RefKind};

/// Default batch size for incremental commit loading.
const BATCH_SIZE: usize = 200;

/// Wrapper around a git2 repository.
pub struct Repo {
    inner: git2::Repository,
    /// Mapping from commit Oid to its reference decorations.
    ref_map: HashMap<git2::Oid, Vec<RefDecoration>>,
    /// Number of commits already yielded (to resume revwalk without re-skipping).
    loaded_count: usize,
}

impl Repo {
    /// Open a git repository at the given path (or discover from it).
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let inner = git2::Repository::discover(path)?;
        let ref_map = Self::build_ref_map(&inner)?;
        Ok(Self {
            inner,
            ref_map,
            loaded_count: 0,
        })
    }

    /// Rebuild the ref decoration map by iterating all references.
    fn build_ref_map(repo: &git2::Repository) -> Result<HashMap<git2::Oid, Vec<RefDecoration>>> {
        let mut map: HashMap<git2::Oid, Vec<RefDecoration>> = HashMap::new();

        // Mark HEAD.
        if let Ok(head) = repo.head()
            && let Some(oid) = head.target()
        {
            map.entry(oid).or_default().push(RefDecoration {
                name: "HEAD".to_string(),
                kind: RefKind::Head,
            });
        }

        // Iterate all references.
        for reference in repo.references()? {
            let reference = match reference {
                Ok(r) => r,
                Err(_) => continue,
            };

            let fullname = match reference.name() {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Resolve to the target commit oid (peel tags).
            let oid = match reference.peel_to_commit() {
                Ok(commit) => commit.id(),
                Err(_) => continue,
            };

            let (name, kind) = if let Some(branch) = fullname.strip_prefix("refs/heads/") {
                (branch.to_string(), RefKind::Branch)
            } else if let Some(remote) = fullname.strip_prefix("refs/remotes/") {
                (remote.to_string(), RefKind::Remote)
            } else if let Some(tag) = fullname.strip_prefix("refs/tags/") {
                (tag.to_string(), RefKind::Tag)
            } else {
                continue;
            };

            map.entry(oid)
                .or_default()
                .push(RefDecoration { name, kind });
        }

        Ok(map)
    }

    /// Load the next batch of commits incrementally.
    /// Returns up to `BATCH_SIZE` commits starting from where the last call left off.
    pub fn load_commits(&mut self) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.inner.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(Sort::TIME)?;

        let commits: Vec<CommitInfo> = revwalk
            .skip(self.loaded_count)
            .take(BATCH_SIZE)
            .filter_map(|oid| oid.ok())
            .filter_map(|oid| {
                let commit = self.inner.find_commit(oid).ok()?;
                let refs = self
                    .ref_map
                    .get(&oid)
                    .map(|v| v.as_slice())
                    .unwrap_or_default()
                    .to_vec();
                Some(CommitInfo {
                    id: oid,
                    summary: commit.summary().unwrap_or("").to_string(),
                    author: commit.author().name().unwrap_or("unknown").to_string(),
                    time: commit.time().seconds(),
                    refs,
                })
            })
            .collect();

        self.loaded_count += commits.len();
        Ok(commits)
    }
}
