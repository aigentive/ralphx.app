// UI feature flag commands — expose runtime config flags to the frontend

use serde::Serialize;

/// Response struct for UI feature flags.
/// Fields use camelCase for frontend consumption via Tauri invoke.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiFeatureFlagsResponse {
    pub activity_page: bool,
    pub extensibility_page: bool,
    pub battle_mode: bool,
}

/// Returns the current UI feature flag configuration.
/// Reads from the OnceLock-cached runtime config; safe to call repeatedly.
#[tauri::command]
pub fn get_ui_feature_flags() -> UiFeatureFlagsResponse {
    let flags = crate::infrastructure::agents::claude::ui_feature_flags_config();
    UiFeatureFlagsResponse {
        activity_page: flags.activity_page,
        extensibility_page: flags.extensibility_page,
        battle_mode: flags.battle_mode,
    }
}

#[cfg(test)]
#[path = "ui_commands_tests.rs"]
mod tests;
