use std::path::PathBuf;

use crate::application::session_turn::use_cases as session_turn_use_cases;
use crate::interface::tauri::dto::{CodexInputItem, CodexTurnRunRequest};
use crate::{RuntimeCodexConfig, SessionTransport};

use super::status_snapshot::StatusSnapshotRequest;

pub(crate) enum SendCodexInputEffect {
    RejectUnsupportedSlash { message: String },
    RenderStatus,
    ForwardTurnRun { request: CodexTurnRunRequest },
}

pub(crate) struct SendCodexInputEffectContext {
    pub(crate) session_id: u64,
    pub(crate) pid: Option<u32>,
    pub(crate) thread_id: Option<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) binary: String,
    pub(crate) transport: SessionTransport,
}

pub(crate) struct SendCodexInputStatusSnapshotPayload {
    pub(crate) session_id: u64,
    pub(crate) pid: Option<u32>,
    pub(crate) thread_id: Option<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) binary: String,
    pub(crate) transport: SessionTransport,
}

impl SendCodexInputStatusSnapshotPayload {
    pub(crate) fn as_snapshot_request<'a>(
        &'a self,
        runtime_config: &'a RuntimeCodexConfig,
    ) -> StatusSnapshotRequest<'a> {
        StatusSnapshotRequest {
            session_id: self.session_id,
            pid: self.pid,
            thread_id: self.thread_id.as_deref(),
            cwd: &self.cwd,
            runtime_config,
            transport: self.transport,
            binary: &self.binary,
        }
    }
}

pub(crate) enum SendCodexInputSideEffect {
    EmitStderr {
        session_id: u64,
        message: String,
    },
    EmitStatusToStdout {
        session_id: u64,
        payload: SendCodexInputStatusSnapshotPayload,
    },
    ScheduleTurnRun {
        request: CodexTurnRunRequest,
    },
}

pub(crate) fn resolve_send_codex_input_effect(
    plan: session_turn_use_cases::SendCodexInputPlan,
    thread_id: Option<String>,
) -> SendCodexInputEffect {
    match plan {
        session_turn_use_cases::SendCodexInputPlan::RejectUnsupportedSlash { message } => {
            SendCodexInputEffect::RejectUnsupportedSlash { message }
        }
        session_turn_use_cases::SendCodexInputPlan::RenderStatus => {
            SendCodexInputEffect::RenderStatus
        }
        session_turn_use_cases::SendCodexInputPlan::ForwardTurnRun { prompt } => {
            SendCodexInputEffect::ForwardTurnRun {
                request: CodexTurnRunRequest {
                    thread_id,
                    input_items: vec![CodexInputItem {
                        item_type: "text".to_string(),
                        text: Some(prompt),
                        path: None,
                        image_url: None,
                        name: None,
                    }],
                    output_schema: None,
                },
            }
        }
    }
}

pub(crate) fn resolve_send_codex_input_side_effect(
    effect: SendCodexInputEffect,
    context: SendCodexInputEffectContext,
) -> SendCodexInputSideEffect {
    match effect {
        SendCodexInputEffect::RejectUnsupportedSlash { message } => {
            SendCodexInputSideEffect::EmitStderr {
                session_id: context.session_id,
                message,
            }
        }
        SendCodexInputEffect::RenderStatus => SendCodexInputSideEffect::EmitStatusToStdout {
            session_id: context.session_id,
            payload: SendCodexInputStatusSnapshotPayload {
                session_id: context.session_id,
                pid: context.pid,
                thread_id: context.thread_id,
                cwd: context.cwd,
                binary: context.binary,
                transport: context.transport,
            },
        },
        SendCodexInputEffect::ForwardTurnRun { request } => {
            SendCodexInputSideEffect::ScheduleTurnRun { request }
        }
    }
}
