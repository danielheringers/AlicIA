use crate::interface::tauri::dto::CodexHelpSnapshot;

const CODEX_HELP_CLI_TREE: &str = r#"codex
  exec (alias: e)
    resume
    review
  review
  login
    status
  logout
  mcp
    list
    get
    add
    remove
    login
    logout
  mcp-server
  app-server
    generate-ts
    generate-json-schema
  completion
  sandbox
    macos
    linux
    windows
  debug
    app-server
      send-message-v2
  apply (alias: a)
  resume
  fork
  cloud
    exec
    status
    list
    apply
    diff
  features
    list
    enable
    disable"#;

const CODEX_HELP_SLASH_COMMANDS: &[&str] = &[
    "/model",
    "/approvals",
    "/permissions",
    "/setup-default-sandbox",
    "/sandbox-add-read-dir",
    "/experimental",
    "/skills",
    "/review",
    "/review-file",
    "/rename",
    "/new",
    "/resume",
    "/fork",
    "/init",
    "/compact",
    "/plan",
    "/collab",
    "/agent",
    "/diff",
    "/mention",
    "/status",
    // "/debug-config",
    "/statusline",
    "/mcp",
    "/apps",
    "/logout",
    "/quit",
    "/exit",
    "/feedback",
    "/ps",
    "/clean",
    "/personality",
    "/debug-m-drop",
    "/debug-m-update",
];

const CODEX_HELP_KEY_FLAGS: &[&str] = &[
    "--model",
    "--image",
    "--profile",
    "--sandbox",
    "--full-auto",
    "--dangerously-bypass-approvals-and-sandbox",
    "--search",
    "--add-dir",
    "--cd",
    "-c key=value",
    "--enable FEATURE",
    "--disable FEATURE",
];

#[tauri::command]
pub fn pick_workspace_folder() -> Option<String> {
    crate::command_runtime::pick_workspace_folder_impl()
}

#[tauri::command]
pub fn pick_image_file() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter(
            "Image",
            &["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"],
        )
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn pick_mention_file() -> Option<String> {
    rfd::FileDialog::new()
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn codex_help_snapshot() -> CodexHelpSnapshot {
    CodexHelpSnapshot {
        cli_tree: CODEX_HELP_CLI_TREE,
        slash_commands: CODEX_HELP_SLASH_COMMANDS.to_vec(),
        key_flags: CODEX_HELP_KEY_FLAGS.to_vec(),
    }
}
