use std::fs;

use reki::model::PatchLineKind;

use crate::support::{TempRepo, commit_details, diffstat_files, git, git_numstat, init_repo};

#[test]
fn commit_details_count_large_text_diffstat_without_rendering_patch() {
    let repo = TempRepo::new();
    init_repo(&repo);

    let large_text = format!("{}\n", "x".repeat(16 * 1024 * 1024 + 1));
    fs::write(repo.path.join("generated.txt"), large_text)
        .expect("large fixture file should be written");
    git(&repo.path, &["add", "."]);
    git(&repo.path, &["commit", "-m", "add generated text"]);

    let details = commit_details(&repo);

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
