// Model ID → display label mapping for RalphX agents.
//
// Catalog construction (2026-04-06):
//   grep -r 'model' src-tauri/src/infrastructure/agents/claude/model_resolver.rs | grep -v '//' | head -40
//   grep -r 'model:' agents/*/claude/agent.yaml agents/*/agent.yaml | grep -v '#' | head -20
//
// Unique model strings found in ralphx.yaml and agent .md files:
//   sonnet, opus, haiku  (short aliases used by all agents in ralphx.yaml)
//
// Full model IDs (claude-*) are included in the table as forward-mapping entries
// for when they appear in runtime --model output or are explicitly set.
//
// Frontend counterpart: frontend/src/lib/model-utils.ts
// When a new model is added to ralphx.yaml or model_resolver.rs, BOTH files must be updated.

/// Map a raw model ID string to a human-readable display label.
///
/// Fallback policy: if the ID is not in the table, the raw ID is returned as-is
/// (so the function never returns an empty string). The caller should provide
/// the raw ID in a tooltip for full-fidelity display.
// Phase 1 uses this when emitting agent:run_started events with model label.
#[allow(dead_code)]
pub(crate) fn model_id_to_label(id: &str) -> String {
    match id {
        // Short aliases used in ralphx.yaml and YAML agent configs
        "sonnet" => "Sonnet 4.6",
        "opus" => "Opus 4.6",
        "haiku" => "Haiku 4.5",
        "gpt-5.4" => "GPT-5.4",
        "gpt-5.4-mini" => "GPT-5.4 Mini",
        "gpt-4.5" => "GPT-4.5",
        // Full model IDs (Claude API format)
        "claude-sonnet-4-6" => "Sonnet 4.6",
        "claude-opus-4-6" => "Opus 4.6",
        "claude-haiku-4-5-20251001" => "Haiku 4.5",
        // Fallback: return raw ID so the chip is never blank
        other => return other.to_string(),
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_alias_labels() {
        assert_eq!(model_id_to_label("sonnet"), "Sonnet 4.6");
        assert_eq!(model_id_to_label("opus"), "Opus 4.6");
        assert_eq!(model_id_to_label("haiku"), "Haiku 4.5");
        assert_eq!(model_id_to_label("gpt-5.4"), "GPT-5.4");
        assert_eq!(model_id_to_label("gpt-5.4-mini"), "GPT-5.4 Mini");
        assert_eq!(model_id_to_label("gpt-4.5"), "GPT-4.5");
    }

    #[test]
    fn test_full_model_id_labels() {
        assert_eq!(model_id_to_label("claude-sonnet-4-6"), "Sonnet 4.6");
        assert_eq!(model_id_to_label("claude-opus-4-6"), "Opus 4.6");
        assert_eq!(model_id_to_label("claude-haiku-4-5-20251001"), "Haiku 4.5");
    }

    #[test]
    fn test_unknown_id_returns_raw() {
        assert_eq!(model_id_to_label("unknown-model"), "unknown-model");
        assert_eq!(model_id_to_label("z-ai/glm-4.7"), "z-ai/glm-4.7");
        assert_eq!(model_id_to_label(""), "");
    }

    /// Drift-prevention test: every model value in ralphx.yaml must have a distinct
    /// display label (not equal to the raw ID). This catches missing entries when
    /// ralphx.yaml gains new model aliases.
    ///
    /// Run: cargo nextest run --manifest-path src-tauri/Cargo.toml --lib -E 'test(test_all_yaml_models_have_labels)'
    #[test]
    fn test_all_yaml_models_have_labels() {
        // Locate ralphx.yaml relative to this crate's manifest directory.
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let yaml_path = manifest_dir.join("../ralphx.yaml");

        let content = match std::fs::read_to_string(&yaml_path) {
            Ok(c) => c,
            Err(e) => {
                // If ralphx.yaml is missing (e.g. CI isolation), skip gracefully.
                eprintln!(
                    "Skipping test_all_yaml_models_have_labels: could not read ralphx.yaml: {e}"
                );
                return;
            }
        };

        // Extract `model: <value>` lines (simple string scan — no full YAML parse needed).
        let models: std::collections::HashSet<&str> = content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                // Match lines like `model: sonnet` or `model: opus`
                if let Some(rest) = trimmed.strip_prefix("model:") {
                    let val = rest.trim();
                    // Skip template placeholders like <SUBAGENT_MODEL_CAP>
                    if val.starts_with('<') {
                        return None;
                    }
                    if !val.is_empty() {
                        return Some(val);
                    }
                }
                None
            })
            .collect();

        assert!(
            !models.is_empty(),
            "No model values found in ralphx.yaml — check file path or format"
        );

        for model_id in &models {
            let label = model_id_to_label(model_id);
            assert_ne!(
                &label, model_id,
                "model_id_to_label({model_id:?}) returned the raw ID — add it to the mapping table in model_labels.rs and frontend/src/lib/model-utils.ts"
            );
            assert!(
                !label.is_empty(),
                "model_id_to_label({model_id:?}) returned an empty label"
            );
        }
    }
}
