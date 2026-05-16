use std::collections::HashMap;

use anyhow::Result;
use gix::bstr::ByteSlice;
use gix::traverse::commit::simple::CommitTimeOrder;

use super::commit::{CommitInfo, RefDecoration, RefKind};
use super::patch::{commit_diffstat, commit_patch};
use crate::model::{CommitDetails, CommitId, CommitSignature};

const OBJECT_CACHE_BYTES: usize = 32 * 1024 * 1024;

/// Wrapper around a Git repository.
pub struct Repo {
    inner: gix::Repository,
    /// Mapping from commit Oid to its reference decorations.
    ref_map: HashMap<CommitId, Vec<RefDecoration>>,
}

/// Incremental cursor over commits reachable from HEAD.
pub(crate) struct CommitCursor<'repo> {
    repo: &'repo Repo,
    walk: gix::revision::Walk<'repo>,
}

impl Repo {
    /// Open a git repository at the given path (or discover from it).
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let mut inner = gix::discover(path)?;
        inner.object_cache_size_if_unset(OBJECT_CACHE_BYTES);

        let ref_map = Self::build_ref_map(&inner)?;
        Ok(Self { inner, ref_map })
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

    /// Create a cursor for incrementally loading commits from HEAD.
    pub(crate) fn commit_cursor(&self) -> Result<CommitCursor<'_>> {
        let walk = self
            .inner
            .head_commit()?
            .ancestors()
            .sorting(gix::revision::walk::Sorting::ByCommitTime(
                CommitTimeOrder::NewestFirst,
            ))
            .all()?;

        Ok(CommitCursor { repo: self, walk })
    }

    pub fn commit_details(&self, id: &CommitId) -> Result<CommitDetails> {
        let object_id = gix::ObjectId::from_hex(id.to_string().as_bytes())?;
        let commit = self.inner.find_commit(object_id)?;
        let info = self.commit_info(&commit);
        let message = commit
            .message_raw()
            .map(|message| message.to_string())
            .unwrap_or_default();
        let diffstat = commit_diffstat(&self.inner, &commit)?;
        let patch = commit_patch(&self.inner, &commit)?;
        let author = model_signature(commit.author()?.into());
        let committer = model_signature(commit.committer()?.into());

        Ok(CommitDetails {
            row: info.to_row(String::new()),
            author,
            committer,
            message,
            diffstat,
            patch,
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
}

impl CommitCursor<'_> {
    /// Load up to `limit` commits from the current cursor position.
    pub(crate) fn next_batch(&mut self, limit: usize) -> Result<Vec<CommitInfo>> {
        let commits = self
            .walk
            .by_ref()
            .filter_map(|info| info.ok())
            .filter_map(|info| {
                let commit = info.object().ok()?;
                Some(self.repo.commit_info(&commit))
            })
            .take(limit)
            .collect();

        Ok(commits)
    }
}

fn model_signature(signature: gix::actor::Signature) -> CommitSignature {
    CommitSignature {
        name: signature.name.to_str_lossy().into_owned(),
        email: signature.email.to_str_lossy().into_owned(),
        time: signature.time.seconds,
        offset_seconds: signature.time.offset,
    }
}

#[cfg(test)]
mod tests;
