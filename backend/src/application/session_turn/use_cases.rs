use crate::domain::session_turn::slash_command_policy::{
    classify_slash_prompt, unsupported_slash_command_message, SlashPromptClassification,
};
use crate::domain::session_turn::SendCodexInputPlan;

const EMPTY_PROMPT_ERROR: &str = "cannot send empty input";

pub(crate) fn plan_send_codex_input(text: &str) -> Result<SendCodexInputPlan, String> {
    let prompt = normalize_send_codex_input_prompt(text);
    if prompt.trim().is_empty() {
        return Err(EMPTY_PROMPT_ERROR.to_string());
    }

    let plan = match classify_slash_prompt(&prompt) {
        SlashPromptClassification::NotSlash => SendCodexInputPlan::ForwardTurnRun { prompt },
        SlashPromptClassification::Status => SendCodexInputPlan::RenderStatus,
        SlashPromptClassification::UnsupportedSlash { command } => {
            SendCodexInputPlan::RejectUnsupportedSlash {
                message: unsupported_slash_command_message(command),
            }
        }
    };
    Ok(plan)
}

fn normalize_send_codex_input_prompt(text: &str) -> String {
    text.trim_end_matches(['\r', '\n']).to_string()
}

#[cfg(test)]
mod tests {
    use super::plan_send_codex_input;
    use crate::domain::session_turn::SendCodexInputPlan;

    #[test]
    fn empty_prompt_returns_validation_error() {
        let plan = plan_send_codex_input("\n \r\n");
        assert_eq!(plan, Err("cannot send empty input".to_string()));
    }

    #[test]
    fn unsupported_slash_preserves_current_error_message() {
        let plan = plan_send_codex_input("/foo bar");
        assert_eq!(
            plan,
            Ok(SendCodexInputPlan::RejectUnsupportedSlash {
                message: "slash command `/foo` is not available in the current runtime. Supported command: /status".to_string(),
            })
        );
    }

    #[test]
    fn status_slash_is_case_insensitive() {
        let plan = plan_send_codex_input(" /STATUS ");
        assert_eq!(plan, Ok(SendCodexInputPlan::RenderStatus));
    }

    #[test]
    fn normal_prompt_forwards_turn_run_with_normalized_text() {
        let plan = plan_send_codex_input("hello world\r\n");
        assert_eq!(
            plan,
            Ok(SendCodexInputPlan::ForwardTurnRun {
                prompt: "hello world".to_string(),
            })
        );
    }
}
