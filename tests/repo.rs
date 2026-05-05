use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use reki::{
    git::Repo,
    model::{CommitId, PatchLineKind},
};

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

fn init_repo(repo: &TempRepo) {
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

#[test]
fn commit_details_report_message_and_diffstat() {
    let repo = TempRepo::new();
    init_repo(&repo);

    fs::create_dir_all(repo.path.join("src/git")).expect("nested fixture directory should exist");
    fs::write(repo.path.join("src/model.rs"), "one\n").expect("fixture file should be written");
    fs::write(repo.path.join("src/git/mod.rs"), "mod repo;\n")
        .expect("nested fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "initial"]);

    fs::write(repo.path.join("src/model.rs"), "two\n").expect("fixture file should be updated");
    fs::write(repo.path.join("src/git/patch.rs"), "pub fn patch() {}\n")
        .expect("second fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "update", "-m", "body"]);
    let head = git(&repo.path, &["rev-parse", "HEAD"]);

    let details = Repo::open(&repo.path)
        .expect("repo should open")
        .commit_details(&CommitId::new(head.trim()))
        .expect("details should load");

    assert_eq!(details.row.summary, "update");
    assert_eq!(details.author.email, "test@example.com");
    assert_eq!(details.committer.email, "test@example.com");
    assert!(details.message.contains("body"));
    assert_eq!(details.diffstat.files_changed, 2);
    assert_eq!(details.diffstat.insertions, 2);
    assert_eq!(details.diffstat.deletions, 1);
    assert_eq!(diffstat_files(&details), git_numstat(&repo.path));
    assert!(
        details
            .diffstat
            .files
            .iter()
            .all(|file| file.path != "src" && file.path != "src/git")
    );

    assert!(details.patch.lines.iter().any(|line| {
        line.kind == PatchLineKind::FileHeader
            && line
                .text
                .contains("diff --git a/src/model.rs b/src/model.rs")
    }));
    assert!(details.patch.lines.iter().any(|line| {
        line.kind == PatchLineKind::FileHeader
            && line
                .text
                .contains("diff --git a/src/git/patch.rs b/src/git/patch.rs")
    }));
    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| line.kind == PatchLineKind::Meta && line.text == "--- a/src/model.rs")
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| line.kind == PatchLineKind::Meta && line.text == "+++ b/src/model.rs")
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| line.kind == PatchLineKind::HunkHeader && line.text.starts_with("@@"))
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| line.kind == PatchLineKind::Addition && line.text == "+two")
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .all(|line| !line.text.contains("diff --git a/src b/src")
                && !line.text.contains("diff --git a/src/git b/src/git"))
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .all(|line| line.text != "Unsupported file type")
    );
}

#[test]
fn commit_details_count_large_text_diffstat_without_rendering_patch() {
    let repo = TempRepo::new();
    init_repo(&repo);

    let large_text = format!("{}\n", "x".repeat(16 * 1024 * 1024 + 1));
    fs::write(repo.path.join("generated.txt"), large_text)
        .expect("large fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "add generated text"]);
    let head = git(&repo.path, &["rev-parse", "HEAD"]);

    let details = Repo::open(&repo.path)
        .expect("repo should open")
        .commit_details(&CommitId::new(head.trim()))
        .expect("details should load");

    assert_eq!(details.diffstat.files_changed, 1);
    assert_eq!(details.diffstat.insertions, 1);
    assert_eq!(details.diffstat.deletions, 0);
    assert_eq!(diffstat_files(&details), git_numstat(&repo.path));
    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| line.text == "Patch omitted: file exceeds 1 MB display limit")
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .all(|line| line.text != "Binary files differ")
    );
}

#[test]
fn commit_details_caps_large_single_hunk_patch() {
    let repo = TempRepo::new();
    init_repo(&repo);

    let generated = (0..5_000)
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    fs::write(repo.path.join("generated.txt"), generated).expect("fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "add generated text"]);

    let details = commit_details(&repo);

    assert_eq!(details.diffstat.insertions, 5_000);
    assert_eq!(details.diffstat.deletions, 0);
    assert!(details.patch.lines.len() <= 4_001);
    assert!(details.patch.lines.iter().any(|line| {
        line.kind == PatchLineKind::Meta
            && line.text == "Patch truncated after 4000 lines, 200 files, or 1000000 bytes"
    }));
}

#[test]
fn commit_details_caps_total_patch_bytes() {
    let repo = TempRepo::new();
    init_repo(&repo);

    let line = "x".repeat(400_000);
    fs::write(
        repo.path.join("generated.txt"),
        format!("{line}\n{line}\n{line}\n"),
    )
    .expect("fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "add generated text"]);

    let details = commit_details(&repo);
    let patch_bytes = details
        .patch
        .lines
        .iter()
        .map(|line| line.text.len())
        .sum::<usize>();

    assert!(patch_bytes < 1_001_000);
    assert!(details.patch.lines.iter().any(|line| {
        line.kind == PatchLineKind::Meta
            && line.text == "Patch truncated after 4000 lines, 200 files, or 1000000 bytes"
    }));
}

#[test]
fn commit_details_caps_diffstat_files_without_losing_totals() {
    let repo = TempRepo::new();
    init_repo(&repo);

    for index in 0..205 {
        fs::write(repo.path.join(format!("file-{index:03}.txt")), "x\n")
            .expect("fixture file should be written");
    }
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "add many files"]);

    let details = commit_details(&repo);
    let git_files = git_numstat(&repo.path);

    assert_eq!(details.diffstat.files_changed, git_files.len());
    assert_eq!(details.diffstat.insertions, 205);
    assert_eq!(details.diffstat.deletions, 0);
    assert_eq!(details.diffstat.files.len(), 200);
    assert_eq!(
        diffstat_files(&details),
        git_files.into_iter().take(200).collect::<Vec<_>>()
    );
}

#[test]
fn commit_details_classify_plus_minus_prefixed_content() {
    let repo = TempRepo::new();
    init_repo(&repo);

    fs::write(repo.path.join("file.txt"), "--old\n++old\n")
        .expect("fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "initial"]);

    fs::write(repo.path.join("file.txt"), "--new\n++new\n")
        .expect("fixture file should be updated");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "update prefixed lines"]);

    let details = commit_details(&repo);

    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| { line.kind == PatchLineKind::Deletion && line.text == "---old" })
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| { line.kind == PatchLineKind::Addition && line.text == "+++new" })
    );
}

#[test]
fn commit_details_preserve_pure_rename() {
    let repo = TempRepo::new();
    init_repo(&repo);

    fs::write(repo.path.join("old.txt"), "same\n").expect("fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "initial"]);

    git(&repo.path, &["mv", "old.txt", "new.txt"]);
    git(&repo.path, &["commit", "-m", "rename file"]);
    let head = git(&repo.path, &["rev-parse", "HEAD"]);

    let details = Repo::open(&repo.path)
        .expect("repo should open")
        .commit_details(&CommitId::new(head.trim()))
        .expect("details should load");

    assert_eq!(details.diffstat.files_changed, 1);
    assert_eq!(details.diffstat.insertions, 0);
    assert_eq!(details.diffstat.deletions, 0);
    assert_eq!(details.diffstat.files[0].path, "old.txt => new.txt");
    assert_eq!(
        details
            .patch
            .lines
            .iter()
            .filter(|line| line.kind == PatchLineKind::FileHeader)
            .count(),
        1
    );
    assert!(details.patch.lines.iter().any(|line| {
        line.kind == PatchLineKind::FileHeader && line.text == "diff --git a/old.txt b/new.txt"
    }));
    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| line.kind == PatchLineKind::Meta && line.text == "rename from old.txt")
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| line.kind == PatchLineKind::Meta && line.text == "rename to new.txt")
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .all(|line| !line.text.starts_with("--- ") && !line.text.starts_with("+++ "))
    );
    assert!(!details.patch.lines.iter().any(|line| {
        (line.kind == PatchLineKind::Addition && line.text == "+same")
            || (line.kind == PatchLineKind::Deletion && line.text == "-same")
    }));
}

#[test]
fn commit_details_quotes_patch_paths() {
    let repo = TempRepo::new();
    init_repo(&repo);

    fs::write(repo.path.join("weird\nname.txt"), "x").expect("fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "add weird path"]);

    let details = commit_details(&repo);

    assert_eq!(
        patch_lines(&details),
        git_show(&repo.path, &["--format=", "HEAD"])
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .all(|line| !line.text.contains('\n'))
    );
    assert!(
        details
            .patch
            .lines
            .iter()
            .any(|line| line.text.contains("\"a/weird\\nname.txt\""))
    );
}

#[test]
fn commit_details_disable_textconv_helpers() {
    let repo = TempRepo::new();
    init_repo(&repo);

    git(&repo.path, &["config", "diff.reki.textconv", "false"]);
    fs::write(repo.path.join(".gitattributes"), "*.dat diff=reki\n")
        .expect("attributes file should be written");
    fs::write(repo.path.join("data.dat"), "one\n").expect("fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "initial"]);

    fs::write(repo.path.join("data.dat"), "two\n").expect("fixture file should be updated");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "update data"]);

    let details = commit_details(&repo);
    let patch = patch_lines(&details);

    assert_eq!(diffstat_files(&details), git_numstat(&repo.path));
    assert!(
        patch
            .iter()
            .any(|line| line == "diff --git a/data.dat b/data.dat")
    );
    assert!(patch.iter().any(|line| line == "-one"));
    assert!(patch.iter().any(|line| line == "+two"));
}

#[test]
fn commit_details_render_merge_patch_from_first_parent() {
    let repo = TempRepo::new();
    init_repo(&repo);

    fs::write(repo.path.join("main.txt"), "base\n").expect("fixture file should be written");
    fs::write(repo.path.join("side.txt"), "base\n").expect("fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "initial"]);
    let main_branch = git(&repo.path, &["branch", "--show-current"]);

    git(&repo.path, &["checkout", "-b", "side"]);
    fs::write(repo.path.join("side.txt"), "side\n").expect("side fixture should be updated");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "side change"]);

    git(&repo.path, &["checkout", main_branch.trim()]);
    fs::write(repo.path.join("main.txt"), "main\n").expect("main fixture should be updated");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "main change"]);
    git(
        &repo.path,
        &["merge", "--no-ff", "side", "-m", "merge side"],
    );

    let details = commit_details(&repo);
    let patch = patch_lines(&details);

    assert_eq!(diffstat_files(&details), git_numstat(&repo.path));
    assert!(
        details
            .diffstat
            .files
            .iter()
            .any(|file| { file.path == "side.txt" && file.insertions == 1 && file.deletions == 1 })
    );
    assert!(
        patch
            .iter()
            .any(|line| line == "diff --git a/side.txt b/side.txt")
    );
    assert!(patch.iter().any(|line| line == "-base"));
    assert!(patch.iter().any(|line| line == "+side"));
}

#[test]
fn commit_details_match_git_for_eof_newline_cases() {
    for (name, old, new) in [
        ("add final newline", "same", "same\n"),
        ("remove final newline", "same\n", "same"),
        ("change final line and remove newline", "one\n", "two"),
        ("change final line and add newline", "one", "two\n"),
        ("change earlier line and add newline", "a\nb", "c\nb\n"),
        ("change earlier line and remove newline", "c\nb\n", "a\nb"),
        ("append after unterminated line", "a\nb", "a\nb\nc\n"),
        ("truncate to unterminated line", "a\nb\nc\n", "a\nb"),
        ("append to unterminated line", "a", "a\nb"),
        ("truncate from unterminated line", "a\nb", "a"),
        ("append to unterminated multiline", "a\nb", "a\nb\nc"),
        ("truncate from unterminated multiline", "a\nb\nc", "a\nb"),
        ("insert before final line", "a\nb", "a\nx\nb\n"),
        ("append duplicate final text", "a\nb", "a\nb\nc\nb\n"),
        ("append duplicate unterminated line", "a", "a\na"),
    ] {
        let repo = TempRepo::new();
        init_repo(&repo);
        fs::write(repo.path.join("file.txt"), old).expect("fixture file should be written");
        git(&repo.path, &["add", "."]);
        git(&repo.path, &["commit", "-m", "initial"]);

        fs::write(repo.path.join("file.txt"), new).expect("fixture file should be updated");
        git(&repo.path, &["add", "."]);
        git(&repo.path, &["commit", "-m", name]);

        let details = commit_details(&repo);
        assert_eq!(diffstat_files(&details), git_numstat(&repo.path), "{name}");
        assert_eq!(patch_lines(&details), git_patch(&repo.path), "{name}");
    }
}

#[test]
fn commit_details_match_git_for_zero_length_hunk_ranges() {
    for (name, old, new) in [
        ("root addition", None, Some("one\n")),
        ("truncate to empty", Some("one\n"), Some("")),
        ("delete file", Some("one\n"), None),
    ] {
        let repo = TempRepo::new();
        init_repo(&repo);
        if let Some(content) = old {
            fs::write(repo.path.join("file.txt"), content).expect("fixture file should be written");
            git(&repo.path, &["add", "."]);
            git(&repo.path, &["commit", "-m", "initial"]);
        }

        match new {
            Some(content) => fs::write(repo.path.join("file.txt"), content)
                .expect("fixture file should be updated"),
            None => {
                git(&repo.path, &["rm", "file.txt"]);
            }
        }
        git(&repo.path, &["add", "."]);
        git(&repo.path, &["commit", "-m", name]);

        let details = commit_details(&repo);
        assert_eq!(diffstat_files(&details), git_numstat(&repo.path), "{name}");
        assert_eq!(patch_lines(&details), git_patch(&repo.path), "{name}");
    }
}

fn commit_details(repo: &TempRepo) -> reki::model::CommitDetails {
    let head = git(&repo.path, &["rev-parse", "HEAD"]);
    Repo::open(&repo.path)
        .expect("repo should open")
        .commit_details(&CommitId::new(head.trim()))
        .expect("details should load")
}

fn patch_lines(details: &reki::model::CommitDetails) -> Vec<String> {
    details
        .patch
        .lines
        .iter()
        .map(|line| line.text.clone())
        .collect()
}

fn diffstat_files(details: &reki::model::CommitDetails) -> Vec<(usize, usize, String)> {
    details
        .diffstat
        .files
        .iter()
        .map(|file| (file.insertions, file.deletions, file.path.clone()))
        .collect()
}

fn git_numstat(repo: &Path) -> Vec<(usize, usize, String)> {
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

fn git_patch(repo: &Path) -> Vec<String> {
    git_show(repo, &["--format=", "HEAD", "--", "file.txt"])
        .lines()
        .map(ToString::to_string)
        .collect()
}

fn git_show(repo: &Path, extra_args: &[&str]) -> String {
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
