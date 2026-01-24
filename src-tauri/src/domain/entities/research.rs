// Research entities for the extensibility system
// Support for long-running research agents with configurable depth

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::artifact::{ArtifactId, ArtifactType, ProcessId};

/// A unique identifier for a ResearchProcess
/// (Reuses ProcessId from artifact module for consistency)
pub type ResearchProcessId = ProcessId;

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

/// The research brief - the question and context for research
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchBrief {
    /// The main question to research
    pub question: String,
    /// Optional additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Optional scope limitations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Optional constraints on the research
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
}

impl ResearchBrief {
    /// Creates a new research brief with just a question
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            context: None,
            scope: None,
            constraints: vec![],
        }
    }

    /// Sets the context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Sets the scope
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Adds a constraint
    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }

    /// Adds multiple constraints
    pub fn with_constraints(mut self, constraints: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.constraints.extend(constraints.into_iter().map(|c| c.into()));
        self
    }
}

/// The output configuration for a research process
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchOutput {
    /// The bucket to store output artifacts in
    pub target_bucket: String,
    /// The types of artifacts this research produces
    #[serde(default)]
    pub artifact_types: Vec<ArtifactType>,
}

impl ResearchOutput {
    /// Creates a new output configuration
    pub fn new(target_bucket: impl Into<String>) -> Self {
        Self {
            target_bucket: target_bucket.into(),
            artifact_types: vec![],
        }
    }

    /// Adds an artifact type to the output
    pub fn with_artifact_type(mut self, artifact_type: ArtifactType) -> Self {
        if !self.artifact_types.contains(&artifact_type) {
            self.artifact_types.push(artifact_type);
        }
        self
    }

    /// Adds multiple artifact types
    pub fn with_artifact_types(mut self, types: impl IntoIterator<Item = ArtifactType>) -> Self {
        for t in types {
            if !self.artifact_types.contains(&t) {
                self.artifact_types.push(t);
            }
        }
        self
    }
}

impl Default for ResearchOutput {
    fn default() -> Self {
        Self::new("research-outputs")
            .with_artifact_types([
                ArtifactType::ResearchDocument,
                ArtifactType::Findings,
                ArtifactType::Recommendations,
            ])
    }
}

/// Progress tracking for a research process
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchProgress {
    /// Current iteration number
    pub current_iteration: u32,
    /// Current status
    pub status: ResearchProcessStatus,
    /// ID of the last checkpoint artifact (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checkpoint: Option<ArtifactId>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl ResearchProgress {
    /// Creates new progress in pending state
    pub fn new() -> Self {
        Self {
            current_iteration: 0,
            status: ResearchProcessStatus::Pending,
            last_checkpoint: None,
            error_message: None,
        }
    }

    /// Sets the status to running
    pub fn start(&mut self) {
        self.status = ResearchProcessStatus::Running;
    }

    /// Increments the iteration counter
    pub fn advance(&mut self) {
        self.current_iteration += 1;
    }

    /// Pauses the process
    pub fn pause(&mut self) {
        self.status = ResearchProcessStatus::Paused;
    }

    /// Resumes from paused state
    pub fn resume(&mut self) {
        self.status = ResearchProcessStatus::Running;
    }

    /// Marks as completed
    pub fn complete(&mut self) {
        self.status = ResearchProcessStatus::Completed;
    }

    /// Marks as failed with an error message
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = ResearchProcessStatus::Failed;
        self.error_message = Some(error.into());
    }

    /// Sets the last checkpoint
    pub fn checkpoint(&mut self, artifact_id: ArtifactId) {
        self.last_checkpoint = Some(artifact_id);
    }

    /// Returns the progress percentage based on max iterations
    pub fn percentage(&self, max_iterations: u32) -> f32 {
        if max_iterations == 0 {
            return 0.0;
        }
        (self.current_iteration as f32 / max_iterations as f32 * 100.0).min(100.0)
    }
}

impl Default for ResearchProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// A research process - a long-running research agent with configurable depth
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchProcess {
    /// Unique identifier
    pub id: ResearchProcessId,
    /// Display name
    pub name: String,
    /// The research brief (question, context, scope, constraints)
    pub brief: ResearchBrief,
    /// Depth configuration (preset or custom)
    pub depth: ResearchDepth,
    /// Agent profile ID to use for this research
    pub agent_profile_id: String,
    /// Output configuration
    pub output: ResearchOutput,
    /// Progress tracking
    pub progress: ResearchProgress,
    /// When the process was created
    pub created_at: DateTime<Utc>,
    /// When the process was started (if started)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// When the process was completed (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

impl ResearchProcess {
    /// Creates a new research process
    pub fn new(
        name: impl Into<String>,
        brief: ResearchBrief,
        agent_profile_id: impl Into<String>,
    ) -> Self {
        Self {
            id: ResearchProcessId::new(),
            name: name.into(),
            brief,
            depth: ResearchDepth::default(),
            agent_profile_id: agent_profile_id.into(),
            output: ResearchOutput::default(),
            progress: ResearchProgress::new(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
        }
    }

    /// Sets the depth configuration
    pub fn with_depth(mut self, depth: ResearchDepth) -> Self {
        self.depth = depth;
        self
    }

    /// Sets the depth to a preset
    pub fn with_preset(mut self, preset: ResearchDepthPreset) -> Self {
        self.depth = ResearchDepth::Preset(preset);
        self
    }

    /// Sets the depth to a custom configuration
    pub fn with_custom_depth(mut self, depth: CustomDepth) -> Self {
        self.depth = ResearchDepth::Custom(depth);
        self
    }

    /// Sets the output configuration
    pub fn with_output(mut self, output: ResearchOutput) -> Self {
        self.output = output;
        self
    }

    /// Gets the resolved depth configuration
    pub fn resolved_depth(&self) -> CustomDepth {
        self.depth.resolve()
    }

    /// Starts the research process
    pub fn start(&mut self) {
        self.progress.start();
        self.started_at = Some(Utc::now());
    }

    /// Advances the iteration counter
    pub fn advance(&mut self) {
        self.progress.advance();
    }

    /// Pauses the research process
    pub fn pause(&mut self) {
        self.progress.pause();
    }

    /// Resumes the research process
    pub fn resume(&mut self) {
        self.progress.resume();
    }

    /// Completes the research process
    pub fn complete(&mut self) {
        self.progress.complete();
        self.completed_at = Some(Utc::now());
    }

    /// Fails the research process
    pub fn fail(&mut self, error: impl Into<String>) {
        self.progress.fail(error);
        self.completed_at = Some(Utc::now());
    }

    /// Creates a checkpoint
    pub fn checkpoint(&mut self, artifact_id: ArtifactId) {
        self.progress.checkpoint(artifact_id);
    }

    /// Returns the current status
    pub fn status(&self) -> ResearchProcessStatus {
        self.progress.status
    }

    /// Returns true if the process is active
    pub fn is_active(&self) -> bool {
        self.progress.status.is_active()
    }

    /// Returns true if the process is complete (finished or failed)
    pub fn is_terminal(&self) -> bool {
        self.progress.status.is_terminal()
    }

    /// Returns the progress percentage
    pub fn progress_percentage(&self) -> f32 {
        let max = self.resolved_depth().max_iterations;
        self.progress.percentage(max)
    }

    /// Returns true if a checkpoint should be saved at the current iteration
    pub fn should_checkpoint(&self) -> bool {
        let interval = self.resolved_depth().checkpoint_interval;
        if interval == 0 {
            return false;
        }
        self.progress.current_iteration > 0 && self.progress.current_iteration % interval == 0
    }

    /// Returns true if the process has reached max iterations
    pub fn is_max_iterations_reached(&self) -> bool {
        self.progress.current_iteration >= self.resolved_depth().max_iterations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== ResearchDepthPreset Tests =====

    #[test]
    fn research_depth_preset_all_returns_4_presets() {
        let all = ResearchDepthPreset::all();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn research_depth_preset_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&ResearchDepthPreset::QuickScan).unwrap(),
            "\"quick-scan\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchDepthPreset::Standard).unwrap(),
            "\"standard\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchDepthPreset::DeepDive).unwrap(),
            "\"deep-dive\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchDepthPreset::Exhaustive).unwrap(),
            "\"exhaustive\""
        );
    }

    #[test]
    fn research_depth_preset_deserializes() {
        let p: ResearchDepthPreset = serde_json::from_str("\"quick-scan\"").unwrap();
        assert_eq!(p, ResearchDepthPreset::QuickScan);
        let p: ResearchDepthPreset = serde_json::from_str("\"deep-dive\"").unwrap();
        assert_eq!(p, ResearchDepthPreset::DeepDive);
    }

    #[test]
    fn research_depth_preset_from_str() {
        assert_eq!(
            ResearchDepthPreset::from_str("quick-scan").unwrap(),
            ResearchDepthPreset::QuickScan
        );
        assert_eq!(
            ResearchDepthPreset::from_str("standard").unwrap(),
            ResearchDepthPreset::Standard
        );
        assert_eq!(
            ResearchDepthPreset::from_str("deep-dive").unwrap(),
            ResearchDepthPreset::DeepDive
        );
        assert_eq!(
            ResearchDepthPreset::from_str("exhaustive").unwrap(),
            ResearchDepthPreset::Exhaustive
        );
    }

    #[test]
    fn research_depth_preset_from_str_error() {
        let err = ResearchDepthPreset::from_str("invalid").unwrap_err();
        assert_eq!(err.value, "invalid");
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn research_depth_preset_display() {
        assert_eq!(ResearchDepthPreset::QuickScan.to_string(), "quick-scan");
        assert_eq!(ResearchDepthPreset::Standard.to_string(), "standard");
        assert_eq!(ResearchDepthPreset::DeepDive.to_string(), "deep-dive");
        assert_eq!(ResearchDepthPreset::Exhaustive.to_string(), "exhaustive");
    }

    #[test]
    fn research_depth_preset_to_custom_depth() {
        let depth = ResearchDepthPreset::QuickScan.to_custom_depth();
        assert_eq!(depth.max_iterations, 10);
        assert_eq!(depth.timeout_hours, 0.5);
        assert_eq!(depth.checkpoint_interval, 5);
    }

    // ===== CustomDepth Tests =====

    #[test]
    fn custom_depth_new_creates_correctly() {
        let depth = CustomDepth::new(100, 4.0, 20);
        assert_eq!(depth.max_iterations, 100);
        assert_eq!(depth.timeout_hours, 4.0);
        assert_eq!(depth.checkpoint_interval, 20);
    }

    #[test]
    fn custom_depth_presets() {
        let quick = CustomDepth::quick_scan();
        assert_eq!(quick.max_iterations, 10);
        assert_eq!(quick.timeout_hours, 0.5);
        assert_eq!(quick.checkpoint_interval, 5);

        let standard = CustomDepth::standard();
        assert_eq!(standard.max_iterations, 50);
        assert_eq!(standard.timeout_hours, 2.0);
        assert_eq!(standard.checkpoint_interval, 10);

        let deep = CustomDepth::deep_dive();
        assert_eq!(deep.max_iterations, 200);
        assert_eq!(deep.timeout_hours, 8.0);
        assert_eq!(deep.checkpoint_interval, 25);

        let exhaustive = CustomDepth::exhaustive();
        assert_eq!(exhaustive.max_iterations, 500);
        assert_eq!(exhaustive.timeout_hours, 24.0);
        assert_eq!(exhaustive.checkpoint_interval, 50);
    }

    #[test]
    fn custom_depth_default_is_standard() {
        let default = CustomDepth::default();
        assert_eq!(default, CustomDepth::standard());
    }

    #[test]
    fn custom_depth_serializes() {
        let depth = CustomDepth::new(100, 4.0, 20);
        let json = serde_json::to_string(&depth).unwrap();
        assert!(json.contains("\"max_iterations\":100"));
        assert!(json.contains("\"timeout_hours\":4.0"));
        assert!(json.contains("\"checkpoint_interval\":20"));
    }

    #[test]
    fn custom_depth_deserializes() {
        let json = r#"{"max_iterations":75,"timeout_hours":3.5,"checkpoint_interval":15}"#;
        let depth: CustomDepth = serde_json::from_str(json).unwrap();
        assert_eq!(depth.max_iterations, 75);
        assert_eq!(depth.timeout_hours, 3.5);
        assert_eq!(depth.checkpoint_interval, 15);
    }

    // ===== RESEARCH_PRESETS Tests =====

    #[test]
    fn research_presets_index_quick_scan() {
        let depth = &RESEARCH_PRESETS[&ResearchDepthPreset::QuickScan];
        assert_eq!(depth.max_iterations, 10);
        assert_eq!(depth.timeout_hours, 0.5);
        assert_eq!(depth.checkpoint_interval, 5);
    }

    #[test]
    fn research_presets_index_standard() {
        let depth = &RESEARCH_PRESETS[&ResearchDepthPreset::Standard];
        assert_eq!(depth.max_iterations, 50);
        assert_eq!(depth.timeout_hours, 2.0);
        assert_eq!(depth.checkpoint_interval, 10);
    }

    #[test]
    fn research_presets_index_deep_dive() {
        let depth = &RESEARCH_PRESETS[&ResearchDepthPreset::DeepDive];
        assert_eq!(depth.max_iterations, 200);
        assert_eq!(depth.timeout_hours, 8.0);
        assert_eq!(depth.checkpoint_interval, 25);
    }

    #[test]
    fn research_presets_index_exhaustive() {
        let depth = &RESEARCH_PRESETS[&ResearchDepthPreset::Exhaustive];
        assert_eq!(depth.max_iterations, 500);
        assert_eq!(depth.timeout_hours, 24.0);
        assert_eq!(depth.checkpoint_interval, 50);
    }

    #[test]
    fn research_presets_get_helper() {
        let depth = ResearchPresets::get(&ResearchDepthPreset::Standard);
        assert_eq!(depth.max_iterations, 50);
    }

    // ===== ResearchDepth Tests =====

    #[test]
    fn research_depth_preset_creates_correctly() {
        let depth = ResearchDepth::preset(ResearchDepthPreset::DeepDive);
        assert!(depth.is_preset());
        assert!(!depth.is_custom());
    }

    #[test]
    fn research_depth_custom_creates_correctly() {
        let depth = ResearchDepth::custom(CustomDepth::new(150, 5.0, 30));
        assert!(depth.is_custom());
        assert!(!depth.is_preset());
    }

    #[test]
    fn research_depth_resolve_preset() {
        let depth = ResearchDepth::preset(ResearchDepthPreset::QuickScan);
        let resolved = depth.resolve();
        assert_eq!(resolved.max_iterations, 10);
        assert_eq!(resolved.timeout_hours, 0.5);
    }

    #[test]
    fn research_depth_resolve_custom() {
        let custom = CustomDepth::new(150, 5.0, 30);
        let depth = ResearchDepth::custom(custom);
        let resolved = depth.resolve();
        assert_eq!(resolved.max_iterations, 150);
        assert_eq!(resolved.timeout_hours, 5.0);
    }

    #[test]
    fn research_depth_default_is_standard_preset() {
        let default = ResearchDepth::default();
        assert!(default.is_preset());
        if let ResearchDepth::Preset(p) = default {
            assert_eq!(p, ResearchDepthPreset::Standard);
        } else {
            panic!("Expected preset");
        }
    }

    #[test]
    fn research_depth_serializes_preset() {
        let depth = ResearchDepth::preset(ResearchDepthPreset::DeepDive);
        let json = serde_json::to_string(&depth).unwrap();
        assert_eq!(json, "\"deep-dive\"");
    }

    #[test]
    fn research_depth_serializes_custom() {
        let depth = ResearchDepth::custom(CustomDepth::new(100, 4.0, 20));
        let json = serde_json::to_string(&depth).unwrap();
        assert!(json.contains("\"max_iterations\":100"));
    }

    #[test]
    fn research_depth_deserializes_preset() {
        let json = "\"quick-scan\"";
        let depth: ResearchDepth = serde_json::from_str(json).unwrap();
        assert!(depth.is_preset());
    }

    #[test]
    fn research_depth_deserializes_custom() {
        let json = r#"{"max_iterations":100,"timeout_hours":4.0,"checkpoint_interval":20}"#;
        let depth: ResearchDepth = serde_json::from_str(json).unwrap();
        assert!(depth.is_custom());
    }

    // ===== ResearchProcessStatus Tests =====

    #[test]
    fn research_process_status_all_returns_5_statuses() {
        let all = ResearchProcessStatus::all();
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn research_process_status_serializes() {
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Paused).unwrap(),
            "\"paused\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&ResearchProcessStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn research_process_status_deserializes() {
        let s: ResearchProcessStatus = serde_json::from_str("\"running\"").unwrap();
        assert_eq!(s, ResearchProcessStatus::Running);
    }

    #[test]
    fn research_process_status_from_str() {
        assert_eq!(
            ResearchProcessStatus::from_str("pending").unwrap(),
            ResearchProcessStatus::Pending
        );
        assert_eq!(
            ResearchProcessStatus::from_str("running").unwrap(),
            ResearchProcessStatus::Running
        );
        assert_eq!(
            ResearchProcessStatus::from_str("paused").unwrap(),
            ResearchProcessStatus::Paused
        );
        assert_eq!(
            ResearchProcessStatus::from_str("completed").unwrap(),
            ResearchProcessStatus::Completed
        );
        assert_eq!(
            ResearchProcessStatus::from_str("failed").unwrap(),
            ResearchProcessStatus::Failed
        );
    }

    #[test]
    fn research_process_status_from_str_error() {
        let err = ResearchProcessStatus::from_str("invalid").unwrap_err();
        assert_eq!(err.value, "invalid");
    }

    #[test]
    fn research_process_status_is_active() {
        assert!(ResearchProcessStatus::Pending.is_active());
        assert!(ResearchProcessStatus::Running.is_active());
        assert!(!ResearchProcessStatus::Paused.is_active());
        assert!(!ResearchProcessStatus::Completed.is_active());
        assert!(!ResearchProcessStatus::Failed.is_active());
    }

    #[test]
    fn research_process_status_is_terminal() {
        assert!(!ResearchProcessStatus::Pending.is_terminal());
        assert!(!ResearchProcessStatus::Running.is_terminal());
        assert!(!ResearchProcessStatus::Paused.is_terminal());
        assert!(ResearchProcessStatus::Completed.is_terminal());
        assert!(ResearchProcessStatus::Failed.is_terminal());
    }

    #[test]
    fn research_process_status_display() {
        assert_eq!(ResearchProcessStatus::Running.to_string(), "running");
        assert_eq!(ResearchProcessStatus::Completed.to_string(), "completed");
    }

    // ===== ResearchBrief Tests =====

    #[test]
    fn research_brief_new_creates_with_question() {
        let brief = ResearchBrief::new("What is the best architecture?");
        assert_eq!(brief.question, "What is the best architecture?");
        assert!(brief.context.is_none());
        assert!(brief.scope.is_none());
        assert!(brief.constraints.is_empty());
    }

    #[test]
    fn research_brief_with_context() {
        let brief = ResearchBrief::new("Question")
            .with_context("Context info");
        assert_eq!(brief.context, Some("Context info".to_string()));
    }

    #[test]
    fn research_brief_with_scope() {
        let brief = ResearchBrief::new("Question")
            .with_scope("Backend only");
        assert_eq!(brief.scope, Some("Backend only".to_string()));
    }

    #[test]
    fn research_brief_with_constraint() {
        let brief = ResearchBrief::new("Question")
            .with_constraint("Must be fast")
            .with_constraint("Must be secure");
        assert_eq!(brief.constraints.len(), 2);
        assert!(brief.constraints.contains(&"Must be fast".to_string()));
        assert!(brief.constraints.contains(&"Must be secure".to_string()));
    }

    #[test]
    fn research_brief_with_constraints() {
        let brief = ResearchBrief::new("Question")
            .with_constraints(["Constraint 1", "Constraint 2"]);
        assert_eq!(brief.constraints.len(), 2);
    }

    #[test]
    fn research_brief_serializes() {
        let brief = ResearchBrief::new("Question")
            .with_context("Context")
            .with_constraint("Constraint");
        let json = serde_json::to_string(&brief).unwrap();
        assert!(json.contains("\"question\":\"Question\""));
        assert!(json.contains("\"context\":\"Context\""));
        assert!(json.contains("\"constraints\":[\"Constraint\"]"));
    }

    #[test]
    fn research_brief_deserializes() {
        let json = r#"{"question":"Test question","context":"Test context"}"#;
        let brief: ResearchBrief = serde_json::from_str(json).unwrap();
        assert_eq!(brief.question, "Test question");
        assert_eq!(brief.context, Some("Test context".to_string()));
    }

    // ===== ResearchOutput Tests =====

    #[test]
    fn research_output_new_creates_correctly() {
        let output = ResearchOutput::new("my-bucket");
        assert_eq!(output.target_bucket, "my-bucket");
        assert!(output.artifact_types.is_empty());
    }

    #[test]
    fn research_output_with_artifact_type() {
        let output = ResearchOutput::new("bucket")
            .with_artifact_type(ArtifactType::Findings)
            .with_artifact_type(ArtifactType::Recommendations);
        assert_eq!(output.artifact_types.len(), 2);
    }

    #[test]
    fn research_output_with_artifact_type_no_duplicates() {
        let output = ResearchOutput::new("bucket")
            .with_artifact_type(ArtifactType::Findings)
            .with_artifact_type(ArtifactType::Findings);
        assert_eq!(output.artifact_types.len(), 1);
    }

    #[test]
    fn research_output_default_has_research_outputs_bucket() {
        let output = ResearchOutput::default();
        assert_eq!(output.target_bucket, "research-outputs");
        assert!(output.artifact_types.contains(&ArtifactType::ResearchDocument));
        assert!(output.artifact_types.contains(&ArtifactType::Findings));
        assert!(output.artifact_types.contains(&ArtifactType::Recommendations));
    }

    #[test]
    fn research_output_serializes() {
        let output = ResearchOutput::new("bucket")
            .with_artifact_type(ArtifactType::Findings);
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"target_bucket\":\"bucket\""));
        assert!(json.contains("\"findings\""));
    }

    // ===== ResearchProgress Tests =====

    #[test]
    fn research_progress_new_is_pending() {
        let progress = ResearchProgress::new();
        assert_eq!(progress.current_iteration, 0);
        assert_eq!(progress.status, ResearchProcessStatus::Pending);
        assert!(progress.last_checkpoint.is_none());
        assert!(progress.error_message.is_none());
    }

    #[test]
    fn research_progress_start() {
        let mut progress = ResearchProgress::new();
        progress.start();
        assert_eq!(progress.status, ResearchProcessStatus::Running);
    }

    #[test]
    fn research_progress_advance() {
        let mut progress = ResearchProgress::new();
        progress.advance();
        assert_eq!(progress.current_iteration, 1);
        progress.advance();
        assert_eq!(progress.current_iteration, 2);
    }

    #[test]
    fn research_progress_pause_resume() {
        let mut progress = ResearchProgress::new();
        progress.start();
        progress.pause();
        assert_eq!(progress.status, ResearchProcessStatus::Paused);
        progress.resume();
        assert_eq!(progress.status, ResearchProcessStatus::Running);
    }

    #[test]
    fn research_progress_complete() {
        let mut progress = ResearchProgress::new();
        progress.start();
        progress.complete();
        assert_eq!(progress.status, ResearchProcessStatus::Completed);
    }

    #[test]
    fn research_progress_fail() {
        let mut progress = ResearchProgress::new();
        progress.start();
        progress.fail("Something went wrong");
        assert_eq!(progress.status, ResearchProcessStatus::Failed);
        assert_eq!(progress.error_message, Some("Something went wrong".to_string()));
    }

    #[test]
    fn research_progress_checkpoint() {
        let mut progress = ResearchProgress::new();
        let artifact_id = ArtifactId::from_string("checkpoint-1");
        progress.checkpoint(artifact_id.clone());
        assert_eq!(progress.last_checkpoint, Some(artifact_id));
    }

    #[test]
    fn research_progress_percentage() {
        let mut progress = ResearchProgress::new();
        assert_eq!(progress.percentage(100), 0.0);
        progress.current_iteration = 25;
        assert_eq!(progress.percentage(100), 25.0);
        progress.current_iteration = 50;
        assert_eq!(progress.percentage(100), 50.0);
        progress.current_iteration = 100;
        assert_eq!(progress.percentage(100), 100.0);
    }

    #[test]
    fn research_progress_percentage_over_max() {
        let mut progress = ResearchProgress::new();
        progress.current_iteration = 150;
        assert_eq!(progress.percentage(100), 100.0);
    }

    #[test]
    fn research_progress_percentage_zero_max() {
        let progress = ResearchProgress::new();
        assert_eq!(progress.percentage(0), 0.0);
    }

    #[test]
    fn research_progress_serializes() {
        let mut progress = ResearchProgress::new();
        progress.start();
        progress.current_iteration = 10;
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("\"current_iteration\":10"));
        assert!(json.contains("\"status\":\"running\""));
    }

    // ===== ResearchProcess Tests =====

    #[test]
    fn research_process_new_creates_correctly() {
        let brief = ResearchBrief::new("What framework to use?");
        let process = ResearchProcess::new("Framework Research", brief, "deep-researcher");
        assert_eq!(process.name, "Framework Research");
        assert_eq!(process.agent_profile_id, "deep-researcher");
        assert_eq!(process.brief.question, "What framework to use?");
        assert!(process.depth.is_preset());
        assert_eq!(process.progress.status, ResearchProcessStatus::Pending);
        assert!(process.started_at.is_none());
        assert!(process.completed_at.is_none());
    }

    #[test]
    fn research_process_with_depth() {
        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_depth(ResearchDepth::preset(ResearchDepthPreset::DeepDive));
        let resolved = process.resolved_depth();
        assert_eq!(resolved.max_iterations, 200);
    }

    #[test]
    fn research_process_with_preset() {
        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::Exhaustive);
        let resolved = process.resolved_depth();
        assert_eq!(resolved.max_iterations, 500);
    }

    #[test]
    fn research_process_with_custom_depth() {
        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_custom_depth(CustomDepth::new(150, 5.0, 30));
        let resolved = process.resolved_depth();
        assert_eq!(resolved.max_iterations, 150);
    }

    #[test]
    fn research_process_with_output() {
        let brief = ResearchBrief::new("Question");
        let output = ResearchOutput::new("custom-bucket")
            .with_artifact_type(ArtifactType::Findings);
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_output(output);
        assert_eq!(process.output.target_bucket, "custom-bucket");
    }

    #[test]
    fn research_process_lifecycle() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan);

        // Initial state
        assert_eq!(process.status(), ResearchProcessStatus::Pending);
        assert!(process.started_at.is_none());

        // Start
        process.start();
        assert_eq!(process.status(), ResearchProcessStatus::Running);
        assert!(process.started_at.is_some());
        assert!(process.is_active());

        // Advance
        process.advance();
        assert_eq!(process.progress.current_iteration, 1);

        // Pause
        process.pause();
        assert_eq!(process.status(), ResearchProcessStatus::Paused);
        assert!(!process.is_active());

        // Resume
        process.resume();
        assert_eq!(process.status(), ResearchProcessStatus::Running);

        // Complete
        process.complete();
        assert_eq!(process.status(), ResearchProcessStatus::Completed);
        assert!(process.is_terminal());
        assert!(process.completed_at.is_some());
    }

    #[test]
    fn research_process_fail() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        process.fail("Network error");
        assert_eq!(process.status(), ResearchProcessStatus::Failed);
        assert!(process.is_terminal());
        assert_eq!(
            process.progress.error_message,
            Some("Network error".to_string())
        );
    }

    #[test]
    fn research_process_checkpoint() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent");
        let artifact_id = ArtifactId::from_string("checkpoint-artifact");
        process.checkpoint(artifact_id.clone());
        assert_eq!(process.progress.last_checkpoint, Some(artifact_id));
    }

    #[test]
    fn research_process_progress_percentage() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan); // 10 max iterations
        assert_eq!(process.progress_percentage(), 0.0);
        process.progress.current_iteration = 5;
        assert_eq!(process.progress_percentage(), 50.0);
        process.progress.current_iteration = 10;
        assert_eq!(process.progress_percentage(), 100.0);
    }

    #[test]
    fn research_process_should_checkpoint() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan); // checkpoint_interval = 5

        process.progress.current_iteration = 0;
        assert!(!process.should_checkpoint());

        process.progress.current_iteration = 4;
        assert!(!process.should_checkpoint());

        process.progress.current_iteration = 5;
        assert!(process.should_checkpoint());

        process.progress.current_iteration = 10;
        assert!(process.should_checkpoint());

        process.progress.current_iteration = 7;
        assert!(!process.should_checkpoint());
    }

    #[test]
    fn research_process_is_max_iterations_reached() {
        let brief = ResearchBrief::new("Question");
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan); // 10 max iterations

        process.progress.current_iteration = 5;
        assert!(!process.is_max_iterations_reached());

        process.progress.current_iteration = 10;
        assert!(process.is_max_iterations_reached());

        process.progress.current_iteration = 15;
        assert!(process.is_max_iterations_reached());
    }

    #[test]
    fn research_process_serializes_roundtrip() {
        let brief = ResearchBrief::new("What is the best approach?")
            .with_context("Building a new feature")
            .with_constraint("Must be maintainable");
        let process = ResearchProcess::new("Architecture Research", brief, "deep-researcher")
            .with_preset(ResearchDepthPreset::DeepDive);

        let json = serde_json::to_string(&process).unwrap();
        let parsed: ResearchProcess = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "Architecture Research");
        assert_eq!(parsed.agent_profile_id, "deep-researcher");
        assert_eq!(parsed.brief.question, "What is the best approach?");
        assert!(parsed.depth.is_preset());
    }

    #[test]
    fn research_process_serializes_with_custom_depth() {
        let brief = ResearchBrief::new("Question");
        let process = ResearchProcess::new("Test", brief, "agent")
            .with_custom_depth(CustomDepth::new(150, 5.0, 30));

        let json = serde_json::to_string(&process).unwrap();
        let parsed: ResearchProcess = serde_json::from_str(&json).unwrap();

        assert!(parsed.depth.is_custom());
        let resolved = parsed.resolved_depth();
        assert_eq!(resolved.max_iterations, 150);
    }
}
