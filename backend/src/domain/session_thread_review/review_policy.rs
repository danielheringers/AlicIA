use serde_json::Value;

pub(crate) fn validate_review_target(target: Option<&Value>) -> Result<(), String> {
    let Some(target) = target else {
        return Ok(());
    };

    let Some(target_object) = target.as_object() else {
        return Err("target must be a plain JSON object".to_string());
    };

    let is_files_target = target_object
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|target_type| target_type == "files");
    if !is_files_target {
        return Ok(());
    }

    let Some(paths) = target_object.get("paths").and_then(Value::as_array) else {
        return Err(
            "target.paths must be a non-empty array when target.type is `files`".to_string(),
        );
    };

    if paths.is_empty() {
        return Err(
            "target.paths must be a non-empty array when target.type is `files`".to_string(),
        );
    }

    for (index, path) in paths.iter().enumerate() {
        let Some(path) = path.as_str() else {
            return Err(format!(
                "target.paths[{index}] must be a non-empty string when target.type is `files`"
            ));
        };

        if path.trim().is_empty() {
            return Err(format!(
                "target.paths[{index}] must be a non-empty string when target.type is `files`"
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_review_delivery(delivery: Option<&str>) -> Result<(), String> {
    if let Some(delivery) = delivery {
        let normalized = delivery.trim().to_ascii_lowercase();
        if !normalized.is_empty() && normalized != "inline" && normalized != "detached" {
            return Err("delivery must be `inline` or `detached`".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_review_start(
    target: Option<&Value>,
    delivery: Option<&str>,
) -> Result<(), String> {
    validate_review_target(target)?;
    validate_review_delivery(delivery)
}

#[cfg(test)]
mod tests {
    use super::{validate_review_delivery, validate_review_start, validate_review_target};
    use serde_json::json;

    #[test]
    fn validate_review_target_rejects_non_object_target() {
        let result = validate_review_target(Some(&json!("invalid")));
        assert_eq!(
            result,
            Err("target must be a plain JSON object".to_string())
        );
    }

    #[test]
    fn validate_review_target_keeps_existing_target_types_unchanged() {
        let result = validate_review_target(Some(&json!({
            "type": "uncommittedChanges"
        })));
        assert!(result.is_ok());
    }

    #[test]
    fn validate_review_target_requires_paths_for_files_target() {
        let result = validate_review_target(Some(&json!({ "type": "files" })));
        assert_eq!(
            result,
            Err("target.paths must be a non-empty array when target.type is `files`".to_string())
        );
    }

    #[test]
    fn validate_review_target_requires_non_empty_trimmed_path_values() {
        let result = validate_review_target(Some(&json!({
            "type": "files",
            "paths": ["src/main.rs", "   "]
        })));
        assert_eq!(
            result,
            Err(
                "target.paths[1] must be a non-empty string when target.type is `files`"
                    .to_string()
            )
        );
    }

    #[test]
    fn validate_review_target_accepts_files_target_with_paths() {
        let result = validate_review_target(Some(&json!({
            "type": "files",
            "paths": ["src/main.rs", "src/session_turn_runtime.rs"]
        })));
        assert!(result.is_ok());
    }

    #[test]
    fn validate_review_delivery_rejects_invalid_value() {
        let result = validate_review_delivery(Some("mail"));
        assert_eq!(
            result,
            Err("delivery must be `inline` or `detached`".to_string())
        );
    }

    #[test]
    fn validate_review_delivery_accepts_empty_inline_and_detached() {
        assert!(validate_review_delivery(None).is_ok());
        assert!(validate_review_delivery(Some("   ")).is_ok());
        assert!(validate_review_delivery(Some("INLINE")).is_ok());
        assert!(validate_review_delivery(Some(" detached ")).is_ok());
    }

    #[test]
    fn validate_review_start_applies_target_and_delivery_rules() {
        let result = validate_review_start(
            Some(&json!({
                "type": "files",
                "paths": ["src/main.rs"]
            })),
            Some("inline"),
        );
        assert!(result.is_ok());
    }
}
