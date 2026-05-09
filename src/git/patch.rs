use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};

use crate::model::{DiffStat, DiffStatFile, Patch, PatchLine, PatchLineKind};

const MAX_PATCH_FILES: usize = 200;
const MAX_PATCH_LINES: usize = 4_000;
const MAX_PATCH_BYTES: usize = 1_000_000;
const MAX_PATCH_LINE_BYTES: usize = 1_000_000;
const MAX_DIFFSTAT_FILES: usize = 200;
const GIT_REPO_ENV_VARS: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_NAMESPACE",
];
const GIT_NO_LAZY_FETCH: &str = "GIT_NO_LAZY_FETCH";

pub(crate) fn commit_patch(repo: &gix::Repository, commit: &gix::Commit<'_>) -> Result<Patch> {
    let commit_id = commit.id.to_string();
    let mut child = git_show_command(repo)
        .args(["--format=", &commit_id])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to spawn git show for commit patch")?;

    let stdout = child
        .stdout
        .take()
        .context("git show stdout should be piped")?;
    let mut reader = BufReader::new(stdout);
    let mut lines = Vec::new();
    let mut files_seen = 0;
    let mut bytes_seen = 0usize;
    let mut truncation_message = None;
    let mut line = Vec::new();
    let mut classifier = PatchLineClassifier::default();

    loop {
        let line = match read_patch_line(&mut reader, &mut line)? {
            PatchReadLine::Eof => break,
            PatchReadLine::Line(line) => line,
            PatchReadLine::TooLong => {
                truncation_message =
                    Some("Patch omitted: file exceeds 1 MB display limit".to_string());
                break;
            }
        };
        if line.starts_with("diff --git ") {
            files_seen += 1;
            if files_seen > MAX_PATCH_FILES {
                truncation_message = Some(patch_truncated_message());
                break;
            }
        }
        if lines.len() >= MAX_PATCH_LINES {
            truncation_message = Some(patch_truncated_message());
            break;
        }
        if bytes_seen.saturating_add(line.len()) > MAX_PATCH_BYTES {
            truncation_message = Some(patch_truncated_message());
            break;
        }

        bytes_seen += line.len();
        lines.push(classifier.classify(&line));
    }

    let was_truncated = truncation_message.is_some();
    if let Some(message) = truncation_message {
        lines.push(PatchLine {
            kind: PatchLineKind::Meta,
            text: message,
        });
        let _ = child.kill();
    }

    let status = child.wait().context("failed to wait for git show")?;
    if !was_truncated && !status.success() {
        bail!("git show failed while rendering commit patch");
    }

    Ok(Patch { lines })
}

fn patch_truncated_message() -> String {
    format!(
        "Patch truncated after {MAX_PATCH_LINES} lines, {MAX_PATCH_FILES} files, or {MAX_PATCH_BYTES} bytes"
    )
}

pub(crate) fn commit_diffstat(
    repo: &gix::Repository,
    commit: &gix::Commit<'_>,
) -> Result<DiffStat> {
    let commit_id = commit.id.to_string();
    let mut child = git_show_command(repo)
        .args(["--numstat", "--format=", &commit_id])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to run git show for commit diffstat")?;

    let stdout = child
        .stdout
        .take()
        .context("git show stdout should be piped")?;
    let mut reader = BufReader::new(stdout);
    let mut line = Vec::new();
    let mut files = Vec::new();
    let mut files_changed = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    loop {
        let line = match read_patch_line(&mut reader, &mut line)? {
            PatchReadLine::Eof => break,
            PatchReadLine::Line(line) => line,
            PatchReadLine::TooLong => {
                let _ = child.kill();
                let _ = child.wait();
                bail!("git show produced a numstat line over 1 MB");
            }
        };
        let Some(file) = parse_numstat_line(&line) else {
            continue;
        };

        files_changed += 1;
        insertions += file.insertions;
        deletions += file.deletions;
        if files.len() < MAX_DIFFSTAT_FILES {
            files.push(file);
        }
    }

    let status = child
        .wait()
        .context("failed to wait for git show diffstat")?;
    if !status.success() {
        bail!("git show failed while rendering commit diffstat");
    }

    Ok(DiffStat {
        files_changed,
        insertions,
        deletions,
        files,
    })
}

fn git_show_command(repo: &gix::Repository) -> Command {
    let mut command = Command::new("git");
    configure_git_environment(&mut command);
    command
        .arg("-C")
        .arg(repo.workdir().unwrap_or_else(|| repo.git_dir()))
        .args([
            "show",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--find-renames",
            "--first-parent",
        ]);
    command
}

fn configure_git_environment(command: &mut Command) {
    for variable in GIT_REPO_ENV_VARS {
        command.env_remove(variable);
    }
    command.env(GIT_NO_LAZY_FETCH, "1");
}

fn parse_numstat_line(line: &str) -> Option<DiffStatFile> {
    let mut fields = line.splitn(3, '\t');
    let insertions = parse_numstat_count(fields.next()?)?;
    let deletions = parse_numstat_count(fields.next()?)?;
    let path = fields.next()?.to_string();

    Some(DiffStatFile {
        path,
        insertions,
        deletions,
    })
}

fn parse_numstat_count(value: &str) -> Option<usize> {
    if value == "-" {
        Some(0)
    } else {
        value.parse().ok()
    }
}

#[derive(Default)]
struct PatchLineClassifier {
    in_hunk: bool,
}

impl PatchLineClassifier {
    fn classify(&mut self, line: &str) -> PatchLine {
        let kind = if line.starts_with("diff --git ") {
            self.in_hunk = false;
            PatchLineKind::FileHeader
        } else if line.starts_with("@@ ") {
            self.in_hunk = true;
            PatchLineKind::HunkHeader
        } else if !self.in_hunk && (line.starts_with("--- ") || line.starts_with("+++ ")) {
            PatchLineKind::Meta
        } else if line.starts_with('+') {
            PatchLineKind::Addition
        } else if line.starts_with('-') {
            PatchLineKind::Deletion
        } else if line.starts_with(' ') {
            PatchLineKind::Context
        } else {
            PatchLineKind::Meta
        };

        PatchLine {
            kind,
            text: line.to_string(),
        }
    }
}

enum PatchReadLine {
    Eof,
    Line(String),
    TooLong,
}

fn read_patch_line<R: BufRead>(reader: &mut R, line: &mut Vec<u8>) -> Result<PatchReadLine> {
    line.clear();

    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if line.is_empty() {
                return Ok(PatchReadLine::Eof);
            }

            return Ok(PatchReadLine::Line(decode_patch_line(line)));
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(available.len(), |index| index + 1);
        let content_bytes = newline.map_or(take, |index| index);
        if line.len().saturating_add(content_bytes) > MAX_PATCH_LINE_BYTES {
            return Ok(PatchReadLine::TooLong);
        }

        line.extend_from_slice(&available[..take]);
        reader.consume(take);

        if newline.is_some() {
            return Ok(PatchReadLine::Line(decode_patch_line(line)));
        }
    }
}

fn decode_patch_line(line: &mut Vec<u8>) -> String {
    if line.ends_with(b"\n") {
        line.pop();
    }
    String::from_utf8_lossy(line).replace('\r', "\\r")
}

#[cfg(test)]
mod tests;
