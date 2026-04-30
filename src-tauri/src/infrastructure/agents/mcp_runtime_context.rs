use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpRuntimeContext {
    pub context_type: Option<String>,
    pub context_id: Option<String>,
    pub task_id: Option<String>,
    pub project_id: Option<String>,
    pub working_directory: Option<PathBuf>,
    pub lead_session_id: Option<String>,
    pub parent_conversation_id: Option<String>,
}

pub fn append_mcp_runtime_query(url: &mut String, runtime_context: Option<&McpRuntimeContext>) {
    let Some(runtime_context) = runtime_context else {
        return;
    };

    let mut params: Vec<(&str, &str)> = Vec::new();
    if let Some(context_type) = runtime_context.context_type.as_deref() {
        params.push(("context_type", context_type));
    }
    if let Some(context_id) = runtime_context.context_id.as_deref() {
        params.push(("context_id", context_id));
    }
    if let Some(project_id) = runtime_context.project_id.as_deref() {
        params.push(("project_id", project_id));
    }
    if let Some(parent_conversation_id) = runtime_context.parent_conversation_id.as_deref() {
        params.push(("parent_conversation_id", parent_conversation_id));
    }

    if params.is_empty() {
        return;
    }

    url.push('?');
    for (index, (key, value)) in params.into_iter().enumerate() {
        if index > 0 {
            url.push('&');
        }
        url.push_str(key);
        url.push('=');
        url.push_str(&encode_query_component(value));
    }
}

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
