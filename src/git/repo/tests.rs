use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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
        let path =
            std::env::temp_dir().join(format!("reki-cursor-test-{}-{stamp}", std::process::id()));
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
