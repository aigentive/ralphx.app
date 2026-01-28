// Type definitions for research system
// Depth presets, status enums, and error types

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Depth presets for research processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResearchDepthPreset {
    /// Fast overview - 10 iterations, 30 min timeout
    QuickScan,
    /// Thorough investigation - 50 iterations, 2 hrs timeout
    Standard,
    /// Comprehensive analysis - 200 iterations, 8 hrs timeout
    DeepDive,
    /// Leave no stone unturned - 500 iterations, 24 hrs timeout
    Exhaustive,
}

impl ResearchDepthPreset {
    /// Returns all depth presets
    pub fn all() -> &'static [ResearchDepthPreset] {
        &[
            ResearchDepthPreset::QuickScan,
            ResearchDepthPreset::Standard,
            ResearchDepthPreset::DeepDive,
            ResearchDepthPreset::Exhaustive,
        ]
    }

    /// Returns the string representation (kebab-case)
    pub fn as_str(&self) -> &'static str {
        match self {
            ResearchDepthPreset::QuickScan => "quick-scan",
            ResearchDepthPreset::Standard => "standard",
            ResearchDepthPreset::DeepDive => "deep-dive",
            ResearchDepthPreset::Exhaustive => "exhaustive",
        }
    }

    /// Converts to CustomDepth configuration
    pub fn to_custom_depth(&self) -> CustomDepth {
        RESEARCH_PRESETS[self]
    }
}

impl fmt::Display for ResearchDepthPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error for parsing ResearchDepthPreset from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResearchDepthPresetError {
    pub value: String,
}

impl fmt::Display for ParseResearchDepthPresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown research depth preset: '{}'", self.value)
    }
}

impl std::error::Error for ParseResearchDepthPresetError {}

impl FromStr for ResearchDepthPreset {
    type Err = ParseResearchDepthPresetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "quick-scan" => Ok(ResearchDepthPreset::QuickScan),
            "standard" => Ok(ResearchDepthPreset::Standard),
            "deep-dive" => Ok(ResearchDepthPreset::DeepDive),
            "exhaustive" => Ok(ResearchDepthPreset::Exhaustive),
            _ => Err(ParseResearchDepthPresetError {
                value: s.to_string(),
            }),
        }
    }
}

/// Custom depth configuration for research processes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CustomDepth {
    /// Maximum number of iterations before stopping
    pub max_iterations: u32,
    /// Maximum time in hours before stopping
    pub timeout_hours: f32,
    /// Save checkpoint every N iterations
    pub checkpoint_interval: u32,
}

impl CustomDepth {
    /// Creates a new custom depth configuration
    pub fn new(max_iterations: u32, timeout_hours: f32, checkpoint_interval: u32) -> Self {
        Self {
            max_iterations,
            timeout_hours,
            checkpoint_interval,
        }
    }

    /// Creates a quick-scan preset (10 iterations, 30 min)
    pub fn quick_scan() -> Self {
        Self::new(10, 0.5, 5)
    }

    /// Creates a standard preset (50 iterations, 2 hrs)
    pub fn standard() -> Self {
        Self::new(50, 2.0, 10)
    }

    /// Creates a deep-dive preset (200 iterations, 8 hrs)
    pub fn deep_dive() -> Self {
        Self::new(200, 8.0, 25)
    }

    /// Creates an exhaustive preset (500 iterations, 24 hrs)
    pub fn exhaustive() -> Self {
        Self::new(500, 24.0, 50)
    }
}

impl Default for CustomDepth {
    fn default() -> Self {
        Self::standard()
    }
}

/// Lookup table for research depth presets
pub struct ResearchPresets;

impl ResearchPresets {
    /// Gets the CustomDepth for a preset
    pub fn get(preset: &ResearchDepthPreset) -> CustomDepth {
        match preset {
            ResearchDepthPreset::QuickScan => CustomDepth::quick_scan(),
            ResearchDepthPreset::Standard => CustomDepth::standard(),
            ResearchDepthPreset::DeepDive => CustomDepth::deep_dive(),
            ResearchDepthPreset::Exhaustive => CustomDepth::exhaustive(),
        }
    }
}

/// Constant providing access to preset configurations
/// Usage: RESEARCH_PRESETS[&ResearchDepthPreset::Standard]
pub const RESEARCH_PRESETS: ResearchPresetsMap = ResearchPresetsMap;

/// Map-like accessor for research presets
pub struct ResearchPresetsMap;

impl std::ops::Index<&ResearchDepthPreset> for ResearchPresetsMap {
    type Output = CustomDepth;

    fn index(&self, preset: &ResearchDepthPreset) -> &Self::Output {
        match preset {
            ResearchDepthPreset::QuickScan => &QUICK_SCAN_DEPTH,
            ResearchDepthPreset::Standard => &STANDARD_DEPTH,
            ResearchDepthPreset::DeepDive => &DEEP_DIVE_DEPTH,
            ResearchDepthPreset::Exhaustive => &EXHAUSTIVE_DEPTH,
        }
    }
}

// Static preset configurations
const QUICK_SCAN_DEPTH: CustomDepth = CustomDepth {
    max_iterations: 10,
    timeout_hours: 0.5,
    checkpoint_interval: 5,
};

const STANDARD_DEPTH: CustomDepth = CustomDepth {
    max_iterations: 50,
    timeout_hours: 2.0,
    checkpoint_interval: 10,
};

const DEEP_DIVE_DEPTH: CustomDepth = CustomDepth {
    max_iterations: 200,
    timeout_hours: 8.0,
    checkpoint_interval: 25,
};

const EXHAUSTIVE_DEPTH: CustomDepth = CustomDepth {
    max_iterations: 500,
    timeout_hours: 24.0,
    checkpoint_interval: 50,
};

/// The depth configuration for a research process - either a preset or custom
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResearchDepth {
    /// Use a predefined preset
    Preset(ResearchDepthPreset),
    /// Use custom configuration
    Custom(CustomDepth),
}

impl ResearchDepth {
    /// Creates a preset depth
    pub fn preset(preset: ResearchDepthPreset) -> Self {
        ResearchDepth::Preset(preset)
    }

    /// Creates a custom depth
    pub fn custom(depth: CustomDepth) -> Self {
        ResearchDepth::Custom(depth)
    }

    /// Resolves to CustomDepth (converts preset to its configuration)
    pub fn resolve(&self) -> CustomDepth {
        match self {
            ResearchDepth::Preset(preset) => preset.to_custom_depth(),
            ResearchDepth::Custom(depth) => *depth,
        }
    }

    /// Returns true if this is a preset
    pub fn is_preset(&self) -> bool {
        matches!(self, ResearchDepth::Preset(_))
    }

    /// Returns true if this is a custom configuration
    pub fn is_custom(&self) -> bool {
        matches!(self, ResearchDepth::Custom(_))
    }
}

impl Default for ResearchDepth {
    fn default() -> Self {
        ResearchDepth::Preset(ResearchDepthPreset::Standard)
    }
}

/// The status of a research process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchProcessStatus {
    /// Not yet started
    Pending,
    /// Currently executing
    Running,
    /// Temporarily paused
    Paused,
    /// Successfully completed
    Completed,
    /// Failed with error
    Failed,
}

impl ResearchProcessStatus {
    /// Returns all statuses
    pub fn all() -> &'static [ResearchProcessStatus] {
        &[
            ResearchProcessStatus::Pending,
            ResearchProcessStatus::Running,
            ResearchProcessStatus::Paused,
            ResearchProcessStatus::Completed,
            ResearchProcessStatus::Failed,
        ]
    }

    /// Returns the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ResearchProcessStatus::Pending => "pending",
            ResearchProcessStatus::Running => "running",
            ResearchProcessStatus::Paused => "paused",
            ResearchProcessStatus::Completed => "completed",
            ResearchProcessStatus::Failed => "failed",
        }
    }

    /// Returns true if the process is active (pending or running)
    pub fn is_active(&self) -> bool {
        matches!(self, ResearchProcessStatus::Pending | ResearchProcessStatus::Running)
    }

    /// Returns true if the process is terminal (completed or failed)
    pub fn is_terminal(&self) -> bool {
        matches!(self, ResearchProcessStatus::Completed | ResearchProcessStatus::Failed)
    }
}

impl fmt::Display for ResearchProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error for parsing ResearchProcessStatus from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResearchProcessStatusError {
    pub value: String,
}

impl fmt::Display for ParseResearchProcessStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown research process status: '{}'", self.value)
    }
}

impl std::error::Error for ParseResearchProcessStatusError {}

impl FromStr for ResearchProcessStatus {
    type Err = ParseResearchProcessStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(ResearchProcessStatus::Pending),
            "running" => Ok(ResearchProcessStatus::Running),
            "paused" => Ok(ResearchProcessStatus::Paused),
            "completed" => Ok(ResearchProcessStatus::Completed),
            "failed" => Ok(ResearchProcessStatus::Failed),
            _ => Err(ParseResearchProcessStatusError {
                value: s.to_string(),
            }),
        }
    }
}
