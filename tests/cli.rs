use std::path::PathBuf;

use reki::cli::{Args, ParseError};

#[test]
fn parses_no_path_as_current_directory_discovery() {
    let args = Args::parse_from(["reki"]).unwrap();

    assert_eq!(args.repo_path, None);
}

#[test]
fn parses_optional_repository_path() {
    let args = Args::parse_from(["reki", "/tmp/repo"]).unwrap();

    assert_eq!(args.repo_path, Some(PathBuf::from("/tmp/repo")));
}

#[test]
fn rejects_extra_arguments() {
    let err = Args::parse_from(["reki", "repo", "extra"]).unwrap_err();

    assert_eq!(err, ParseError::TooManyArguments);
}
