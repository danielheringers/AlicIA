use crate::domain::session_turn::slash_command_policy::{
    classify_slash_prompt, unsupported_slash_command_message, SlashPromptClassification,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SendCodexInputAction {
    ForwardTurnRun,
    RenderStatus,
    RejectUnsupportedSlash { message: String },
}

pub(crate) fn decide_send_codex_input_action(prompt: &str) -> SendCodexInputAction {
    match classify_slash_prompt(prompt) {
        SlashPromptClassification::NotSlash => SendCodexInputAction::ForwardTurnRun,
        SlashPromptClassification::Status => SendCodexInputAction::RenderStatus,
        SlashPromptClassification::UnsupportedSlash { command } => {
            SendCodexInputAction::RejectUnsupportedSlash {
                message: unsupported_slash_command_message(command),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decide_send_codex_input_action, SendCodexInputAction};

    #[test]
    fn unsupported_slash_preserves_current_error_message() {
        let decision = decide_send_codex_input_action("/foo bar");
        assert_eq!(
            decision,
            SendCodexInputAction::RejectUnsupportedSlash {
                message: "slash command `/foo` is not available in the current runtime. Supported command: /status".to_string(),
            }
        );
    }

    #[test]
    fn normal_prompt_forwards_turn_run() {
        let decision = decide_send_codex_input_action("hello world");
        assert_eq!(decision, SendCodexInputAction::ForwardTurnRun);
    }
}
