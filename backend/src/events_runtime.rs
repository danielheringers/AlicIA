use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use crate::generated::runtime_contract::{
    EVENT_CHANNEL_CODEX_EVENT, EVENT_CHANNEL_CODEX_LIFECYCLE, EVENT_CHANNEL_CODEX_STDERR,
    EVENT_CHANNEL_CODEX_STDOUT, EVENT_CHANNEL_TERMINAL_DATA, EVENT_CHANNEL_TERMINAL_EXIT,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamEventPayload {
    session_id: u64,
    chunk: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CodexStructuredEventPayload {
    session_id: u64,
    seq: u64,
    event: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LifecycleEventPayload {
    status: &'static str,
    session_id: Option<u64>,
    pid: Option<u32>,
    exit_code: Option<i32>,
    message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalDataPayload {
    terminal_id: u64,
    seq: u64,
    chunk: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TerminalExitPayload {
    terminal_id: u64,
    seq: u64,
    exit_code: Option<i32>,
}

pub(crate) fn emit_lifecycle(
    app: &AppHandle,
    status: &'static str,
    session_id: Option<u64>,
    pid: Option<u32>,
    exit_code: Option<i32>,
    message: Option<String>,
) {
    let payload = LifecycleEventPayload {
        status,
        session_id,
        pid,
        exit_code,
        message,
    };
    let _ = app.emit(EVENT_CHANNEL_CODEX_LIFECYCLE, payload);
}

fn emit_stream(app: &AppHandle, channel: &str, session_id: u64, chunk: String) {
    let payload = StreamEventPayload { session_id, chunk };
    let _ = app.emit(channel, payload);
}

pub(crate) fn emit_stdout(app: &AppHandle, session_id: u64, chunk: String) {
    emit_stream(app, EVENT_CHANNEL_CODEX_STDOUT, session_id, chunk);
}

pub(crate) fn emit_stderr(app: &AppHandle, session_id: u64, chunk: String) {
    emit_stream(app, EVENT_CHANNEL_CODEX_STDERR, session_id, chunk);
}

pub(crate) fn emit_codex_event(
    app: &AppHandle,
    session_id: u64,
    event: Value,
    event_seq: &Arc<AtomicU64>,
) {
    let seq = event_seq.fetch_add(1, Ordering::Relaxed);
    let payload = CodexStructuredEventPayload {
        session_id,
        seq,
        event,
    };
    let _ = app.emit(EVENT_CHANNEL_CODEX_EVENT, payload);
}

pub(crate) fn emit_terminal_data(
    app: &AppHandle,
    terminal_id: u64,
    event_seq: &Arc<AtomicU64>,
    chunk: String,
) {
    let seq = event_seq.fetch_add(1, Ordering::Relaxed);
    let payload = TerminalDataPayload {
        terminal_id,
        seq,
        chunk,
    };
    let _ = app.emit(EVENT_CHANNEL_TERMINAL_DATA, payload);
}

pub(crate) fn emit_terminal_exit(
    app: &AppHandle,
    terminal_id: u64,
    event_seq: &Arc<AtomicU64>,
    exit_code: Option<i32>,
) {
    let seq = event_seq.fetch_add(1, Ordering::Relaxed);
    let payload = TerminalExitPayload {
        terminal_id,
        seq,
        exit_code,
    };
    let _ = app.emit(EVENT_CHANNEL_TERMINAL_EXIT, payload);
}
