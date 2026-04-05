use super::session_namer_prompt::build_session_namer_prompt;

#[test]
fn session_namer_prompt_preserves_identifier_guidance() {
    let prompt = build_session_namer_prompt(
        "<session_id>abc</session_id>\n<user_message>PDM-301: Standardize payloads</user_message>",
    );

    assert!(
        prompt.contains("preserve it in the title"),
        "prompt should explicitly preserve work-item identifiers"
    );
    assert!(
        prompt.contains("PDM-301"),
        "prompt should include a concrete identifier example"
    );
    assert!(
        prompt.contains("Do not invent identifiers"),
        "prompt should forbid made-up identifiers"
    );
}
