use serde_json::{json, Value};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::Path;
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::resolve_codex_launch;

pub(crate) fn write_json_line(stdin: &mut ChildStdin, payload: &Value) -> Result<(), String> {
    let serialized = serde_json::to_string(payload)
        .map_err(|error| format!("failed to encode json-rpc payload: {error}"))?;
    writeln!(stdin, "{serialized}").map_err(|error| {
        format!("failed to write json-rpc payload to app-server stdin: {error}")
    })?;
    stdin
        .flush()
        .map_err(|error| format!("failed to flush app-server stdin: {error}"))
}

#[cfg(feature = "native-codex-runtime")]
pub(crate) fn request_app_server_method(
    binary: &str,
    cwd: &Path,
    method: &str,
    params: Value,
    timeout: Duration,
) -> Result<Value, String> {
    let app_server_args = vec!["app-server".to_string()];
    let (program, resolved_args) = resolve_codex_launch(binary, &app_server_args)?;

    let mut command = Command::new(program);
    command.args(resolved_args);
    command.current_dir(cwd);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());

    let mut child = command.spawn().map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            format!("failed to spawn app-server for {method}: executable not found ({error})")
        } else {
            format!("failed to spawn app-server for {method}: {error}")
        }
    })?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| format!("failed to capture app-server stdin for {method}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("failed to capture app-server stdout for {method}"))?;

    let (tx, rx) = mpsc::channel::<Value>();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            let read = match reader.read_line(&mut line) {
                Ok(value) => value,
                Err(_) => break,
            };
            if read == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                let _ = tx.send(parsed);
            }
        }
    });

    let request_id = format!("alicia-request-{}", method.replace('/', "-"));
    let result = (|| {
        write_json_line(
            &mut stdin,
            &json!({
                "method": "initialize",
                "id": "alicia-init",
                "params": {
                    "clientInfo": {
                        "name": "alicia-command-runtime",
                        "title": "Alicia Command Runtime",
                        "version": "0.1.0",
                    },
                    "capabilities": {
                        "experimentalApi": false,
                    },
                },
            }),
        )?;
        write_json_line(
            &mut stdin,
            &json!({
                "method": "initialized",
                "params": {},
            }),
        )?;
        write_json_line(
            &mut stdin,
            &json!({
                "method": method,
                "id": request_id.clone(),
                "params": params,
            }),
        )?;

        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(format!(
                    "timed out waiting for app-server `{method}` response"
                ));
            }
            let remaining = deadline.saturating_duration_since(now);
            let message = rx
                .recv_timeout(remaining)
                .map_err(|_| format!("timed out waiting for app-server `{method}` response"))?;

            if message.get("id").and_then(Value::as_str) != Some(request_id.as_str()) {
                continue;
            }

            if let Some(error_value) = message.get("error") {
                let message = error_value
                    .get("message")
                    .and_then(Value::as_str)
                    .or_else(|| error_value.as_str())
                    .unwrap_or("unknown app-server error");
                return Err(format!("app-server `{method}` request failed: {message}"));
            }

            let result = message
                .get("result")
                .cloned()
                .ok_or_else(|| format!("app-server `{method}` response missing `result`"))?;
            return Ok(result);
        }
    })();

    let _ = child.kill();
    let _ = child.wait();
    result
}
