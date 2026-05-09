use std::fs;

use reki::model::PatchLineKind;

use crate::support::{
    TempRepo, commit_details, diffstat_files, git, git_numstat, git_show, init_repo, patch_lines,
};

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

    let details = commit_details(&repo);

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
fn commit_details_preserve_pure_rename() {
    let repo = TempRepo::new();
    init_repo(&repo);

    fs::write(repo.path.join("old.txt"), "same\n").expect("fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "initial"]);

    git(&repo.path, &["mv", "old.txt", "new.txt"]);
    git(&repo.path, &["commit", "-m", "rename file"]);

    let details = commit_details(&repo);

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
