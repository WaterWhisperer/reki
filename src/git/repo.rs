use std::collections::HashMap;

use anyhow::Result;
use gix::traverse::commit::simple::CommitTimeOrder;

use super::commit::{CommitInfo, RefDecoration, RefKind};
use crate::model::{CommitDetails, CommitId, DiffStat};

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

fn saturating_usize(value: u64) -> usize {
    value.try_into().unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::Repo;

    struct TempRepo {
        path: PathBuf,
    }

    impl TempRepo {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("reki-cursor-test-{}-{stamp}", std::process::id()));
            fs::create_dir_all(&path).expect("temp repo directory should be created");
            Self { path }
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn commit_cursor_advances_across_batches_without_repeating_commits() {
        let repo = TempRepo::new();
        git(&repo.path, &["init"]);
        git(&repo.path, &["config", "user.name", "Test User"]);
        git(&repo.path, &["config", "user.email", "test@example.com"]);
        git(&repo.path, &["config", "commit.gpgSign", "false"]);

        for index in 1..=3 {
            fs::write(repo.path.join("a.txt"), format!("{index}\n"))
                .expect("fixture file should be written");
            git(&repo.path, &["add", "."]);
            git(&repo.path, &["commit", "-m", &format!("commit {index}")]);
        }

        let repo = Repo::open(&repo.path).expect("repo should open");
        let mut cursor = repo.commit_cursor().expect("cursor should be created");

        let first = cursor.next_batch(2).expect("first batch should load");
        let second = cursor.next_batch(2).expect("second batch should load");
        let third = cursor.next_batch(2).expect("third batch should load");

        let mut ids = first
            .iter()
            .chain(second.iter())
            .map(|commit| commit.id.to_string())
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();

        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 1);
        assert!(third.is_empty());
        assert_eq!(ids.len(), 3);
    }
}
