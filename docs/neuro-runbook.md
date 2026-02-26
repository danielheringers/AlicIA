# Neuro Runtime Runbook

This runbook consolidates Neuro environment setup, execution, and diagnostics for the Alicia desktop runtime (`alicia/backend`).

## Scope

- Runtime host: Alicia Tauri backend.
- Runtime module: `alicia/backend/src/neuro_runtime.rs`.
- Command bridge: `alicia/frontend/lib/tauri-bridge/commands.ts`.

## 1. Environment Setup

Neuro runtime reads `NEURO_*` variables first, then falls back to legacy `SAP_*` aliases.

### Required

- `NEURO_SAP_URL` (fallback: `SAP_URL`)
- `NEURO_SAP_USER` (fallback: `SAP_USER` or `SAP_USERNAME`)
- `NEURO_SAP_PASSWORD` (fallback: `SAP_PASSWORD` or `SAP_PASS`)

If `NEURO_SAP_URL` is missing, runtime initialization fails with `runtime_init_error`.

### Optional SAP/ADT

- `NEURO_SAP_CLIENT` (fallback: `SAP_CLIENT`)
- `NEURO_SAP_LANGUAGE` (fallback: `SAP_LANGUAGE` or `SAP_LANG`)
- `NEURO_SAP_INSECURE` (fallback: `SAP_INSECURE`, default: `false`)
- `NEURO_SAP_TIMEOUT_SECS` (fallback: `SAP_TIMEOUT_SECS`, default: `30`)
- `NEURO_ADT_CSRF_FETCH_PATH` (fallback: `SAP_ADT_CSRF_FETCH_PATH`, default: `/sap/bc/adt`)
- `NEURO_ADT_SEARCH_PATH` (fallback: `SAP_ADT_SEARCH_PATH`, default: `/sap/bc/adt/repository/informationsystem/search?operation=quickSearch`)

### Optional WebSocket

- `NEURO_WS_URL` (fallback: `SAP_WS_URL`)
- `NEURO_WS_TIMEOUT_SECS` (fallback: `SAP_WS_TIMEOUT_SECS`, default: `15`)
- Header prefixes:
  - `NEURO_WS_HEADER_<NAME>=<value>`
  - `SAP_WS_HEADER_<NAME>=<value>`

Header names are lowercased and `_` is converted to `-`.

### Optional Safety Controls

- `NEURO_SAFETY_READ_ONLY` (default: `false`)
- `NEURO_SAFETY_BLOCKED_PATTERNS` (comma-separated)
- `NEURO_SAFETY_ALLOWED_WS_DOMAINS` (comma-separated)
- `NEURO_UPDATE_REQUIRE_ETAG` (default: `false`)

## 2. Execution

From `alicia/`:

```powershell
pnpm run setup
pnpm run dev
```

This starts Next.js + Tauri backend. Neuro runtime initializes lazily on first Neuro command.

Minimal PowerShell env example before `pnpm run dev`:

```powershell
$env:NEURO_SAP_URL="https://sap.example.internal"
$env:NEURO_SAP_USER="your-user"
$env:NEURO_SAP_PASSWORD="your-password"
```

Optional direct engine smoke test (from `codex/codex-rs`):

```powershell
cargo run -p neuro-cli -- `
  --adt-base-url $env:NEURO_SAP_URL `
  --adt-user $env:NEURO_SAP_USER `
  --adt-password $env:NEURO_SAP_PASSWORD `
  diagnose
```

## 3. Diagnostics

### Runtime diagnose contract

The Tauri command `neuro_runtime_diagnose` returns:

- `overall_status`: `healthy`, `degraded`, or `unavailable`
- `components`: includes `adt_http`, `neuro_ws`, `safety_policy`, plus env checks
- `metadata`: resolved config sources and safety/runtime flags

Bridge helper: `neuroRuntimeDiagnose()` in `alicia/frontend/lib/tauri-bridge/commands.ts`.

### Quick triage order

1. Confirm required env variables are present and non-empty.
2. Check runtime init/log errors in the Tauri backend output.
3. Run diagnose and inspect `components` + `metadata` source fields.
4. Validate ADT credentials/connectivity with `neuro-cli diagnose`.
5. If WebSocket is required, set `NEURO_WS_URL` and verify `neuro_ws` is not `unavailable`.

### Common failures

- `runtime_init_error`: missing/invalid base runtime config (usually URL missing).
- `invalid_argument`: malformed boolean/integer env value.
- `adt_auth_error`: credentials rejected (`401`/`403`).
- `adt_http_error`: ADT HTTP request failed for non-auth reasons.
- `adt_csrf_error`: CSRF token flow failed.
- `ws_unavailable`: WS URL missing or WS client unavailable.
- `ws_timeout`: WS request timeout.
- `safety_violation`: blocked by read-only, ETag requirement, blocked pattern, or domain allowlist.

## 4. Notes

- Prefer `NEURO_*` names in new setups; keep `SAP_*` only for backward compatibility.
- Boolean env parsing accepts `true/false`, `1/0`, `yes/no`, `on/off`.
- Runtime caches Neuro initialization after first successful init.
