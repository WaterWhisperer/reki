use std::collections::HashMap;

use anyhow::Result;
use gix::traverse::commit::simple::CommitTimeOrder;

use super::commit::{CommitInfo, RefDecoration, RefKind};
use crate::model::{CommitDetails, CommitId, DiffStat};

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
                Some(self.commit_info(&commit))
            })
            .collect();

        self.loaded_count += commits.len();
        Ok(commits)
    }

    pub fn commit_details(&self, id: &CommitId) -> Result<CommitDetails> {
        let object_id = gix::ObjectId::from_hex(id.to_string().as_bytes())?;
        let commit = self.inner.find_commit(object_id)?;
        let info = self.commit_info(&commit);
        let message = commit
            .message_raw()
            .map(|message| message.to_string())
            .unwrap_or_default();
        let diffstat = self.diffstat(&commit)?;

        Ok(CommitDetails {
            row: info.to_row(String::new()),
            message,
            diffstat,
        })
    }

    fn commit_info(&self, commit: &gix::Commit<'_>) -> CommitInfo {
        let id = CommitId::new(commit.id.to_string());
        let parent_ids = commit
            .parent_ids()
            .map(|id| CommitId::new(id.to_string()))
            .collect();
        let refs = self
            .ref_map
            .get(&id)
            .map(|v| v.as_slice())
            .unwrap_or_default()
            .to_vec();

        CommitInfo {
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
        }
    }

    fn diffstat(&self, commit: &gix::Commit<'_>) -> Result<DiffStat> {
        let new_tree = commit.tree()?;
        let old_tree = match commit.parent_ids().next() {
            Some(parent_id) => parent_id.object()?.try_into_commit()?.tree()?,
            None => self.inner.empty_tree(),
        };
        let mut changes = old_tree.changes()?;
        changes.options(|options| {
            options.track_rewrites(None);
        });
        let stats = changes.stats(&new_tree)?;

        Ok(DiffStat {
            files_changed: saturating_usize(stats.files_changed),
            insertions: saturating_usize(stats.lines_added),
            deletions: saturating_usize(stats.lines_removed),
        })
    }
}

fn saturating_usize(value: u64) -> usize {
    value.try_into().unwrap_or(usize::MAX)
}
