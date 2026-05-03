use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use reki::{git::Repo, model::CommitId};

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
            std::env::temp_dir().join(format!("reki-repo-test-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path).expect("temp repo directory should be created");
        Self { path }
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn git(repo: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).expect("git output should be utf-8")
}

#[test]
fn commit_details_report_message_and_diffstat() {
    let repo = TempRepo::new();
    git(&repo.path, &["init"]);
    git(&repo.path, &["config", "user.name", "Test User"]);
    git(&repo.path, &["config", "user.email", "test@example.com"]);
    git(&repo.path, &["config", "commit.gpgSign", "false"]);

    fs::write(repo.path.join("a.txt"), "one\n").expect("fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "initial"]);

    fs::write(repo.path.join("a.txt"), "one\ntwo\n").expect("fixture file should be updated");
    fs::write(repo.path.join("b.txt"), "new\n").expect("second fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "update", "-m", "body"]);
    let head = git(&repo.path, &["rev-parse", "HEAD"]);

    let repo = Repo::open(&repo.path).expect("repo should open");
    let details = repo
        .commit_details(&CommitId::new(head.trim()))
        .expect("details should load");

    assert_eq!(details.row.summary, "update");
    assert!(details.message.contains("body"));
    assert_eq!(details.diffstat.files_changed, 2);
    assert_eq!(details.diffstat.insertions, 2);
    assert_eq!(details.diffstat.deletions, 0);
}
