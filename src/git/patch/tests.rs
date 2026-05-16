use std::ffi::OsStr;
use std::process::Command;

use super::{GIT_NO_LAZY_FETCH, GIT_REPO_ENV_VARS, configure_git_environment, decode_patch_line};

#[test]
fn configure_git_environment_isolates_inspect_commands() {
    let mut command = Command::new("git");
    for variable in GIT_REPO_ENV_VARS {
        command.env(variable, "ambient");
    }

    configure_git_environment(&mut command);

    for variable in GIT_REPO_ENV_VARS {
        let value = command
            .get_envs()
            .find(|(name, _)| *name == OsStr::new(variable))
            .and_then(|(_, value)| value);
        assert_eq!(value, None, "{variable} should be removed");
    }

    let value = command
        .get_envs()
        .find(|(name, _)| *name == OsStr::new(GIT_NO_LAZY_FETCH))
        .and_then(|(_, value)| value);
    assert_eq!(value, Some(OsStr::new("1")));
}

#[test]
fn decode_patch_line_preserves_carriage_returns_visibly() {
    let mut line = b"+foo\r\n".to_vec();

    assert_eq!(decode_patch_line(&mut line), "+foo\\r");
}
