use std::collections::HashMap;

use anyhow::Result;
use gix::traverse::commit::simple::CommitTimeOrder;

use super::commit::{CommitInfo, RefDecoration, RefKind};
use crate::model::CommitId;

/// Default batch size for incremental commit loading.
const BATCH_SIZE: usize = 200;

/// Wrapper around a Git repository.
pub struct Repo {
    inner: gix::Repository,
    /// Mapping from commit Oid to its reference decorations.
    ref_map: HashMap<CommitId, Vec<RefDecoration>>,
    /// Number of commits already yielded (to resume revwalk without re-skipping).
    loaded_count: usize,
}

impl Repo {
    /// Open a git repository at the given path (or discover from it).
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let inner = gix::discover(path)?;
        let ref_map = Self::build_ref_map(&inner)?;
        Ok(Self {
            inner,
            ref_map,
            loaded_count: 0,
        })
    }

    /// Rebuild the ref decoration map by iterating all references.
    fn build_ref_map(repo: &gix::Repository) -> Result<HashMap<CommitId, Vec<RefDecoration>>> {
        let mut map: HashMap<CommitId, Vec<RefDecoration>> = HashMap::new();

        // Mark HEAD.
        if let Ok(head) = repo.head_commit() {
            map.entry(CommitId::new(head.id.to_string()))
                .or_default()
                .push(RefDecoration {
                    name: "HEAD".to_string(),
                    kind: RefKind::Head,
                });
        }

        // Iterate all references.
        for reference in repo.references()?.all()? {
            let mut reference = match reference {
                Ok(r) => r,
                Err(_) => continue,
            };

            let fullname = reference.name().as_bstr().to_string();

            // Resolve to the target commit oid (peel tags).
            let id = match reference.peel_to_id() {
                Ok(id) => CommitId::new(id.to_string()),
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

            map.entry(id)
                .or_default()
                .push(RefDecoration { name, kind });
        }

        Ok(map)
    }

    /// Load the next batch of commits incrementally.
    /// Returns up to `BATCH_SIZE` commits starting from where the last call left off.
    pub fn load_commits(&mut self) -> Result<Vec<CommitInfo>> {
        let walk = self
            .inner
            .head_commit()?
            .ancestors()
            .sorting(gix::revision::walk::Sorting::ByCommitTime(
                CommitTimeOrder::NewestFirst,
            ))
            .all()?;

        let commits: Vec<CommitInfo> = walk
            .skip(self.loaded_count)
            .take(BATCH_SIZE)
            .filter_map(|info| info.ok())
            .filter_map(|info| {
                let commit = info.object().ok()?;
                let id = CommitId::new(info.id.to_string());
                let parent_ids: Vec<CommitId> = info
                    .parent_ids()
                    .map(|id| CommitId::new(id.to_string()))
                    .collect();
                let refs = self
                    .ref_map
                    .get(&id)
                    .map(|v| v.as_slice())
                    .unwrap_or_default()
                    .to_vec();
                Some(CommitInfo {
                    id,
                    parent_ids,
                    summary: commit
                        .message()
                        .map(|message| message.title.to_string())
                        .unwrap_or_default(),
                    author: commit
                        .author()
                        .map(|author| author.name.to_string())
                        .unwrap_or_else(|_| "unknown".to_string()),
                    time: commit.time().map(|time| time.seconds).unwrap_or_default(),
                    refs,
                })
            })
            .collect();

        self.loaded_count += commits.len();
        Ok(commits)
    }
}
