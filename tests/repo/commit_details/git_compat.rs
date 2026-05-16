use std::fs;

use reki::model::PatchLineKind;

use crate::support::{
    TempRepo, commit_details, diffstat_files, git, git_numstat, git_patch, init_repo, patch_lines,
};

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
    git(&repo.path, &[
        "merge",
        "--no-ff",
        "side",
        "-m",
        "merge side",
    ]);

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
            },
        }
        git(&repo.path, &["add", "."]);
        git(&repo.path, &["commit", "-m", name]);

        let details = commit_details(&repo);
        assert_eq!(diffstat_files(&details), git_numstat(&repo.path), "{name}");
        assert_eq!(patch_lines(&details), git_patch(&repo.path), "{name}");
    }
}
