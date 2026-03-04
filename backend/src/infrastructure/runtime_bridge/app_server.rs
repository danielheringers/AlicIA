use serde_json::Value;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::{resolve_codex_launch, RunCodexCommandResponse};

pub(crate) fn run_codex_command_with_context(
    binary: &str,
    args: Vec<String>,
    cwd: Option<&Path>,
) -> Result<RunCodexCommandResponse, String> {
    let (program, resolved_args) = resolve_codex_launch(binary, &args)?;

    let mut command = Command::new(program);
    command.args(resolved_args);

    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let output = command.output().map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            format!("failed to run codex command: executable not found ({error})")
        } else {
            format!("failed to run codex command: {error}")
        }
    })?;

    Ok(RunCodexCommandResponse {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        status: output.status.code().unwrap_or(-1),
        success: output.status.success(),
    })
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn request_app_server_method(
    binary: &str,
    cwd: &Path,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<Value, String> {
    crate::app_server_runtime::request_app_server_method(binary, cwd, method, params, timeout)
}

#[cfg(not(feature = "native-codex-runtime"))]
pub(crate) fn request_app_server_method(
    _binary: &str,
    _cwd: &Path,
    _method: &str,
    _params: Value,
    _timeout: Duration,
) -> Result<Value, String> {
    Err("native runtime feature is disabled".to_string())
}
