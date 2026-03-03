use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexHelpSnapshot {
    pub(crate) cli_tree: &'static str,
    pub(crate) slash_commands: Vec<&'static str>,
    pub(crate) key_flags: Vec<&'static str>,
}
