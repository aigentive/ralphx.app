use serde::{Deserialize, Serialize};

/// UI configuration section from ralphx.yaml.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiConfig {
    /// Feature flags controlling page visibility.
    #[serde(default)]
    pub feature_flags: Option<UiFeatureFlagsConfig>,
}

/// Per-page feature flag configuration.
///
/// Defaults to all pages enabled for backward compatibility with configs
/// that do not have a `ui.feature_flags` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiFeatureFlagsConfig {
    /// Show or hide the Activity page. Default: true.
    pub activity_page: bool,
    /// Show or hide the Extensibility page. Default: true.
    pub extensibility_page: bool,
}

impl Default for UiFeatureFlagsConfig {
    fn default() -> Self {
        Self {
            activity_page: true,
            extensibility_page: true,
        }
    }
}
