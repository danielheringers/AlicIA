use std::path::Path;

use crate::status_runtime::{fetch_rate_limits_for_status, format_non_tui_status};
use crate::{RuntimeCodexConfig, SessionTransport};

pub(crate) struct StatusSnapshotRequest<'a> {
    pub(crate) session_id: u64,
    pub(crate) pid: Option<u32>,
    pub(crate) thread_id: Option<&'a str>,
    pub(crate) cwd: &'a Path,
    pub(crate) runtime_config: &'a RuntimeCodexConfig,
    pub(crate) transport: SessionTransport,
    pub(crate) binary: &'a str,
}

pub(crate) fn build_non_tui_status_snapshot(request: StatusSnapshotRequest<'_>) -> String {
    let rate_limits = fetch_rate_limits_for_status(request.binary, request.cwd);
    format_non_tui_status(
        request.session_id,
        request.pid,
        request.thread_id,
        request.cwd,
        request.runtime_config,
        request.transport,
        rate_limits.as_ref(),
    )
}
