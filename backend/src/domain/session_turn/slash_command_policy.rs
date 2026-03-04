#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SlashPromptClassification<'a> {
    NotSlash,
    Status,
    UnsupportedSlash { command: &'a str },
}

pub(crate) fn classify_slash_prompt(prompt: &str) -> SlashPromptClassification<'_> {
    let Some((command, _args)) = parse_slash_command(prompt) else {
        return SlashPromptClassification::NotSlash;
    };

    if command.eq_ignore_ascii_case("/status") {
        SlashPromptClassification::Status
    } else {
        SlashPromptClassification::UnsupportedSlash { command }
    }
}

pub(crate) fn unsupported_slash_command_message(command: &str) -> String {
    let normalized = command.trim();
    let display_command = if normalized.is_empty() {
        "/"
    } else {
        normalized
    };
    format!(
        "slash command `{display_command}` is not available in the current runtime. Supported command: /status"
    )
}

fn parse_slash_command(prompt: &str) -> Option<(&str, &str)> {
    let trimmed = prompt.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let command = parts.next()?;
    let args = parts.next().unwrap_or("").trim();
    Some((command, args))
}

#[cfg(test)]
mod tests {
    use super::{classify_slash_prompt, SlashPromptClassification};

    #[test]
    fn classify_slash_prompt_recognizes_status() {
        assert_eq!(
            classify_slash_prompt("/status"),
            SlashPromptClassification::Status
        );
    }

    #[test]
    fn classify_slash_prompt_recognizes_status_case_insensitive() {
        assert_eq!(
            classify_slash_prompt("/STATUS"),
            SlashPromptClassification::Status
        );
    }
}
