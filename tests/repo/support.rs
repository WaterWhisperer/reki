use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use reki::{
    git::Repo,
    model::{CommitDetails, CommitId},
};

pub(crate) struct TempRepo {
    pub(crate) path: PathBuf,
}

impl TempRepo {
    pub(crate) fn new() -> Self {
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

pub(crate) fn git(repo: &Path, args: &[&str]) -> String {
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

pub(crate) fn init_repo(repo: &TempRepo) {
    git(&repo.path, &["init"]);
    git(&repo.path, &["config", "user.name", "Test User"]);
    git(&repo.path, &["config", "user.email", "test@example.com"]);
    git(&repo.path, &["config", "commit.gpgSign", "false"]);
    git(&repo.path, &["config", "diff.renames", "true"]);
    git(
        &repo.path,
        &["config", "core.bigFileThreshold", "536870912"],
    );
}

pub(crate) fn commit_details(repo: &TempRepo) -> CommitDetails {
    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    Repo::open(&repo.path)
        .expect("repo should open")
        .commit_details(&CommitId::new(head.trim()))
        .expect("details should load")
}

pub(crate) fn patch_lines(details: &CommitDetails) -> Vec<String> {
    details
        .patch
        .lines
        .iter()
        .map(|line| line.text.clone())
        .collect()
}

pub(crate) fn diffstat_files(details: &CommitDetails) -> Vec<(usize, usize, String)> {
    details
        .diffstat
        .files
        .iter()
        .map(|file| (file.insertions, file.deletions, file.path.clone()))
        .collect()
}

pub(crate) fn git_numstat(repo: &Path) -> Vec<(usize, usize, String)> {
    git_show(repo, &["--numstat", "--format=", "HEAD"])
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let insertions = fields.next()?.parse().ok()?;
            let deletions = fields.next()?.parse().ok()?;
            let path = fields.next()?.to_string();
            Some((insertions, deletions, path))
        })
        .collect()
}

pub(crate) fn git_patch(repo: &Path) -> Vec<String> {
    git_show(repo, &["--format=", "HEAD", "--", "file.txt"])
        .lines()
        .map(ToString::to_string)
        .collect()
}

pub(crate) fn git_show(repo: &Path, extra_args: &[&str]) -> String {
    let mut args = vec![
        "show",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--find-renames",
        "--first-parent",
    ];
    args.extend_from_slice(extra_args);
    git(repo, &args)
}
