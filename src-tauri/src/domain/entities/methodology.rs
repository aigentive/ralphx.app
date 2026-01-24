// Methodology entities for the extensibility system
// Support for development methodologies like BMAD and GSD

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::workflow::WorkflowSchema;

/// A unique identifier for a MethodologyExtension
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MethodologyId(pub String);

impl MethodologyId {
    /// Creates a new MethodologyId with a random UUID v4
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Creates a MethodologyId from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Returns the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MethodologyId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MethodologyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A methodology extension - a configuration package that brings workflow, agents, skills, phases
///
/// A methodology is a combination of Workflow + Agents + Artifacts. When a user activates
/// a methodology, the Kanban columns change to reflect that methodology's workflow while
/// still mapping to internal statuses for consistent side effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodologyExtension {
    /// Unique identifier
    pub id: MethodologyId,
    /// Display name for the methodology
    pub name: String,
    /// Description of the methodology
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Agent profiles this methodology provides (profile IDs)
    #[serde(default)]
    pub agent_profiles: Vec<String>,
    /// Skills bundled with methodology (paths to skill directories)
    #[serde(default)]
    pub skills: Vec<String>,
    /// Custom workflow for this methodology
    pub workflow: WorkflowSchema,
    /// Phase/stage definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phases: Vec<MethodologyPhase>,
    /// Document templates
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<MethodologyTemplate>,
    /// Hooks configuration (stored as JSON for flexibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks_config: Option<serde_json::Value>,
    /// Whether this methodology is currently active
    #[serde(default)]
    pub is_active: bool,
    /// When the methodology was created
    pub created_at: DateTime<Utc>,
}

impl MethodologyExtension {
    /// Creates a new methodology extension
    pub fn new(name: impl Into<String>, workflow: WorkflowSchema) -> Self {
        Self {
            id: MethodologyId::new(),
            name: name.into(),
            description: None,
            agent_profiles: vec![],
            skills: vec![],
            workflow,
            phases: vec![],
            templates: vec![],
            hooks_config: None,
            is_active: false,
            created_at: Utc::now(),
        }
    }

    /// Sets the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds an agent profile
    pub fn with_agent_profile(mut self, profile_id: impl Into<String>) -> Self {
        self.agent_profiles.push(profile_id.into());
        self
    }

    /// Adds multiple agent profiles
    pub fn with_agent_profiles(mut self, profiles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.agent_profiles.extend(profiles.into_iter().map(|p| p.into()));
        self
    }

    /// Adds a skill path
    pub fn with_skill(mut self, skill_path: impl Into<String>) -> Self {
        self.skills.push(skill_path.into());
        self
    }

    /// Adds multiple skill paths
    pub fn with_skills(mut self, skills: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.skills.extend(skills.into_iter().map(|s| s.into()));
        self
    }

    /// Adds a phase
    pub fn with_phase(mut self, phase: MethodologyPhase) -> Self {
        self.phases.push(phase);
        self
    }

    /// Adds multiple phases
    pub fn with_phases(mut self, phases: impl IntoIterator<Item = MethodologyPhase>) -> Self {
        self.phases.extend(phases);
        self
    }

    /// Adds a template
    pub fn with_template(mut self, template: MethodologyTemplate) -> Self {
        self.templates.push(template);
        self
    }

    /// Adds multiple templates
    pub fn with_templates(mut self, templates: impl IntoIterator<Item = MethodologyTemplate>) -> Self {
        self.templates.extend(templates);
        self
    }

    /// Sets the hooks configuration
    pub fn with_hooks_config(mut self, hooks: serde_json::Value) -> Self {
        self.hooks_config = Some(hooks);
        self
    }

    /// Marks the methodology as active
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// Marks the methodology as inactive
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Returns the number of phases
    pub fn phase_count(&self) -> usize {
        self.phases.len()
    }

    /// Returns the number of agent profiles
    pub fn agent_count(&self) -> usize {
        self.agent_profiles.len()
    }

    /// Returns phases sorted by order
    pub fn sorted_phases(&self) -> Vec<&MethodologyPhase> {
        let mut sorted: Vec<_> = self.phases.iter().collect();
        sorted.sort_by_key(|p| p.order);
        sorted
    }

    /// Returns the phase at a given order index
    pub fn phase_at_order(&self, order: u32) -> Option<&MethodologyPhase> {
        self.phases.iter().find(|p| p.order == order)
    }
}

/// A phase or stage in a methodology
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodologyPhase {
    /// Unique identifier within the methodology
    pub id: String,
    /// Display name for the phase
    pub name: String,
    /// Order in the phase sequence (0-based)
    pub order: u32,
    /// Agent profile IDs that work in this phase
    #[serde(default)]
    pub agent_profiles: Vec<String>,
    /// Description of the phase
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Column IDs in the workflow that belong to this phase
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_ids: Vec<String>,
}

impl MethodologyPhase {
    /// Creates a new phase
    pub fn new(id: impl Into<String>, name: impl Into<String>, order: u32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            order,
            agent_profiles: vec![],
            description: None,
            column_ids: vec![],
        }
    }

    /// Sets the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds an agent profile
    pub fn with_agent_profile(mut self, profile_id: impl Into<String>) -> Self {
        self.agent_profiles.push(profile_id.into());
        self
    }

    /// Adds multiple agent profiles
    pub fn with_agent_profiles(mut self, profiles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.agent_profiles.extend(profiles.into_iter().map(|p| p.into()));
        self
    }

    /// Adds a column ID
    pub fn with_column(mut self, column_id: impl Into<String>) -> Self {
        self.column_ids.push(column_id.into());
        self
    }

    /// Adds multiple column IDs
    pub fn with_columns(mut self, columns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.column_ids.extend(columns.into_iter().map(|c| c.into()));
        self
    }
}

/// A document template for a methodology
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodologyTemplate {
    /// The artifact type this template produces
    pub artifact_type: String,
    /// Path to the template file (relative to methodology directory)
    pub template_path: String,
    /// Display name for the template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Description of when to use this template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl MethodologyTemplate {
    /// Creates a new template
    pub fn new(artifact_type: impl Into<String>, template_path: impl Into<String>) -> Self {
        Self {
            artifact_type: artifact_type.into(),
            template_path: template_path.into(),
            name: None,
            description: None,
        }
    }

    /// Sets the display name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Status of a methodology in a project
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodologyStatus {
    /// Available but not active
    Available,
    /// Currently active for the project
    Active,
    /// Temporarily disabled
    Disabled,
}

impl MethodologyStatus {
    /// Returns all statuses
    pub fn all() -> &'static [MethodologyStatus] {
        &[
            MethodologyStatus::Available,
            MethodologyStatus::Active,
            MethodologyStatus::Disabled,
        ]
    }

    /// Returns the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            MethodologyStatus::Available => "available",
            MethodologyStatus::Active => "active",
            MethodologyStatus::Disabled => "disabled",
        }
    }
}

impl fmt::Display for MethodologyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error for parsing MethodologyStatus from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMethodologyStatusError {
    pub value: String,
}

impl fmt::Display for ParseMethodologyStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown methodology status: '{}'", self.value)
    }
}

impl std::error::Error for ParseMethodologyStatusError {}

impl FromStr for MethodologyStatus {
    type Err = ParseMethodologyStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "available" => Ok(MethodologyStatus::Available),
            "active" => Ok(MethodologyStatus::Active),
            "disabled" => Ok(MethodologyStatus::Disabled),
            _ => Err(ParseMethodologyStatusError {
                value: s.to_string(),
            }),
        }
    }
}

impl Default for MethodologyStatus {
    fn default() -> Self {
        MethodologyStatus::Available
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::status::InternalStatus;
    use crate::domain::entities::workflow::WorkflowColumn;

    fn create_test_workflow() -> WorkflowSchema {
        WorkflowSchema::new(
            "Test Workflow",
            vec![
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("in_progress", "In Progress", InternalStatus::Executing),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        )
    }

    // ===== MethodologyId Tests =====

    #[test]
    fn methodology_id_new_generates_valid_uuid() {
        let id = MethodologyId::new();
        assert_eq!(id.as_str().len(), 36);
        assert!(uuid::Uuid::parse_str(id.as_str()).is_ok());
    }

    #[test]
    fn methodology_id_from_string_preserves_value() {
        let id = MethodologyId::from_string("bmad-method");
        assert_eq!(id.as_str(), "bmad-method");
    }

    #[test]
    fn methodology_id_equality_works() {
        let id1 = MethodologyId::from_string("method-1");
        let id2 = MethodologyId::from_string("method-1");
        let id3 = MethodologyId::from_string("method-2");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn methodology_id_serializes() {
        let id = MethodologyId::from_string("test-method");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"test-method\"");
    }

    #[test]
    fn methodology_id_deserializes() {
        let json = "\"deserialized-method\"";
        let id: MethodologyId = serde_json::from_str(json).unwrap();
        assert_eq!(id.as_str(), "deserialized-method");
    }

    #[test]
    fn methodology_id_display() {
        let id = MethodologyId::from_string("display-test");
        assert_eq!(id.to_string(), "display-test");
    }

    // ===== MethodologyExtension Tests =====

    #[test]
    fn methodology_extension_new_creates_correctly() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test Method", workflow);
        assert_eq!(methodology.name, "Test Method");
        assert!(!methodology.id.as_str().is_empty());
        assert!(methodology.description.is_none());
        assert!(methodology.agent_profiles.is_empty());
        assert!(methodology.skills.is_empty());
        assert!(methodology.phases.is_empty());
        assert!(methodology.templates.is_empty());
        assert!(methodology.hooks_config.is_none());
        assert!(!methodology.is_active);
    }

    #[test]
    fn methodology_extension_with_description() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_description("A test methodology");
        assert_eq!(methodology.description, Some("A test methodology".to_string()));
    }

    #[test]
    fn methodology_extension_with_agent_profile() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_agent_profile("analyst")
            .with_agent_profile("developer");
        assert_eq!(methodology.agent_profiles.len(), 2);
        assert!(methodology.agent_profiles.contains(&"analyst".to_string()));
        assert!(methodology.agent_profiles.contains(&"developer".to_string()));
    }

    #[test]
    fn methodology_extension_with_agent_profiles() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_agent_profiles(["pm", "architect", "developer"]);
        assert_eq!(methodology.agent_profiles.len(), 3);
    }

    #[test]
    fn methodology_extension_with_skill() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_skill("skills/prd-creation")
            .with_skill("skills/code-review");
        assert_eq!(methodology.skills.len(), 2);
    }

    #[test]
    fn methodology_extension_with_skills() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_skills(["skill1", "skill2", "skill3"]);
        assert_eq!(methodology.skills.len(), 3);
    }

    #[test]
    fn methodology_extension_with_phase() {
        let workflow = create_test_workflow();
        let phase = MethodologyPhase::new("analysis", "Analysis", 0);
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_phase(phase);
        assert_eq!(methodology.phases.len(), 1);
        assert_eq!(methodology.phases[0].name, "Analysis");
    }

    #[test]
    fn methodology_extension_with_phases() {
        let workflow = create_test_workflow();
        let phases = vec![
            MethodologyPhase::new("p1", "Phase 1", 0),
            MethodologyPhase::new("p2", "Phase 2", 1),
        ];
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_phases(phases);
        assert_eq!(methodology.phases.len(), 2);
    }

    #[test]
    fn methodology_extension_with_template() {
        let workflow = create_test_workflow();
        let template = MethodologyTemplate::new("prd", "templates/prd.md");
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_template(template);
        assert_eq!(methodology.templates.len(), 1);
    }

    #[test]
    fn methodology_extension_with_templates() {
        let workflow = create_test_workflow();
        let templates = vec![
            MethodologyTemplate::new("prd", "templates/prd.md"),
            MethodologyTemplate::new("design_doc", "templates/design.md"),
        ];
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_templates(templates);
        assert_eq!(methodology.templates.len(), 2);
    }

    #[test]
    fn methodology_extension_with_hooks_config() {
        let workflow = create_test_workflow();
        let hooks = serde_json::json!({
            "pre_commit": ["validate_prd"],
            "post_review": ["notify_pm"]
        });
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_hooks_config(hooks);
        assert!(methodology.hooks_config.is_some());
    }

    #[test]
    fn methodology_extension_activate_deactivate() {
        let workflow = create_test_workflow();
        let mut methodology = MethodologyExtension::new("Test", workflow);
        assert!(!methodology.is_active);

        methodology.activate();
        assert!(methodology.is_active);

        methodology.deactivate();
        assert!(!methodology.is_active);
    }

    #[test]
    fn methodology_extension_phase_count() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_phase(MethodologyPhase::new("p1", "Phase 1", 0))
            .with_phase(MethodologyPhase::new("p2", "Phase 2", 1));
        assert_eq!(methodology.phase_count(), 2);
    }

    #[test]
    fn methodology_extension_agent_count() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_agent_profiles(["a1", "a2", "a3"]);
        assert_eq!(methodology.agent_count(), 3);
    }

    #[test]
    fn methodology_extension_sorted_phases() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_phase(MethodologyPhase::new("p2", "Phase 2", 2))
            .with_phase(MethodologyPhase::new("p0", "Phase 0", 0))
            .with_phase(MethodologyPhase::new("p1", "Phase 1", 1));

        let sorted = methodology.sorted_phases();
        assert_eq!(sorted[0].id, "p0");
        assert_eq!(sorted[1].id, "p1");
        assert_eq!(sorted[2].id, "p2");
    }

    #[test]
    fn methodology_extension_phase_at_order() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Test", workflow)
            .with_phase(MethodologyPhase::new("analysis", "Analysis", 0))
            .with_phase(MethodologyPhase::new("planning", "Planning", 1));

        assert_eq!(methodology.phase_at_order(0).unwrap().id, "analysis");
        assert_eq!(methodology.phase_at_order(1).unwrap().id, "planning");
        assert!(methodology.phase_at_order(2).is_none());
    }

    #[test]
    fn methodology_extension_serializes_roundtrip() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Roundtrip Test", workflow)
            .with_description("Test description")
            .with_agent_profiles(["agent1", "agent2"])
            .with_skills(["skill1"])
            .with_phase(MethodologyPhase::new("p1", "Phase 1", 0))
            .with_template(MethodologyTemplate::new("prd", "templates/prd.md"));

        let json = serde_json::to_string(&methodology).unwrap();
        let parsed: MethodologyExtension = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "Roundtrip Test");
        assert_eq!(parsed.description, Some("Test description".to_string()));
        assert_eq!(parsed.agent_profiles.len(), 2);
        assert_eq!(parsed.skills.len(), 1);
        assert_eq!(parsed.phases.len(), 1);
        assert_eq!(parsed.templates.len(), 1);
    }

    #[test]
    fn methodology_extension_skips_empty_optional_fields() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Minimal", workflow);
        let json = serde_json::to_string(&methodology).unwrap();

        assert!(!json.contains("\"description\""));
        assert!(!json.contains("\"phases\""));
        assert!(!json.contains("\"templates\""));
        assert!(!json.contains("\"hooks_config\""));
    }

    // ===== MethodologyPhase Tests =====

    #[test]
    fn methodology_phase_new_creates_correctly() {
        let phase = MethodologyPhase::new("analysis", "Analysis Phase", 0);
        assert_eq!(phase.id, "analysis");
        assert_eq!(phase.name, "Analysis Phase");
        assert_eq!(phase.order, 0);
        assert!(phase.agent_profiles.is_empty());
        assert!(phase.description.is_none());
        assert!(phase.column_ids.is_empty());
    }

    #[test]
    fn methodology_phase_with_description() {
        let phase = MethodologyPhase::new("analysis", "Analysis", 0)
            .with_description("Analyze requirements");
        assert_eq!(phase.description, Some("Analyze requirements".to_string()));
    }

    #[test]
    fn methodology_phase_with_agent_profile() {
        let phase = MethodologyPhase::new("analysis", "Analysis", 0)
            .with_agent_profile("analyst");
        assert_eq!(phase.agent_profiles.len(), 1);
        assert!(phase.agent_profiles.contains(&"analyst".to_string()));
    }

    #[test]
    fn methodology_phase_with_agent_profiles() {
        let phase = MethodologyPhase::new("analysis", "Analysis", 0)
            .with_agent_profiles(["analyst", "researcher"]);
        assert_eq!(phase.agent_profiles.len(), 2);
    }

    #[test]
    fn methodology_phase_with_column() {
        let phase = MethodologyPhase::new("analysis", "Analysis", 0)
            .with_column("brainstorm")
            .with_column("research");
        assert_eq!(phase.column_ids.len(), 2);
    }

    #[test]
    fn methodology_phase_with_columns() {
        let phase = MethodologyPhase::new("analysis", "Analysis", 0)
            .with_columns(["col1", "col2", "col3"]);
        assert_eq!(phase.column_ids.len(), 3);
    }

    #[test]
    fn methodology_phase_serializes() {
        let phase = MethodologyPhase::new("test", "Test Phase", 1)
            .with_description("A test phase")
            .with_agent_profile("tester");
        let json = serde_json::to_string(&phase).unwrap();

        assert!(json.contains("\"id\":\"test\""));
        assert!(json.contains("\"name\":\"Test Phase\""));
        assert!(json.contains("\"order\":1"));
        assert!(json.contains("\"description\":\"A test phase\""));
        assert!(json.contains("\"agent_profiles\":[\"tester\"]"));
    }

    #[test]
    fn methodology_phase_deserializes() {
        let json = r#"{"id":"phase1","name":"Phase 1","order":0,"agent_profiles":["agent1"]}"#;
        let phase: MethodologyPhase = serde_json::from_str(json).unwrap();

        assert_eq!(phase.id, "phase1");
        assert_eq!(phase.name, "Phase 1");
        assert_eq!(phase.order, 0);
        assert_eq!(phase.agent_profiles, vec!["agent1"]);
    }

    #[test]
    fn methodology_phase_skips_empty_fields() {
        let phase = MethodologyPhase::new("minimal", "Minimal", 0);
        let json = serde_json::to_string(&phase).unwrap();

        assert!(!json.contains("\"description\""));
        assert!(!json.contains("\"column_ids\""));
    }

    // ===== MethodologyTemplate Tests =====

    #[test]
    fn methodology_template_new_creates_correctly() {
        let template = MethodologyTemplate::new("prd", "templates/prd.md");
        assert_eq!(template.artifact_type, "prd");
        assert_eq!(template.template_path, "templates/prd.md");
        assert!(template.name.is_none());
        assert!(template.description.is_none());
    }

    #[test]
    fn methodology_template_with_name() {
        let template = MethodologyTemplate::new("prd", "templates/prd.md")
            .with_name("PRD Template");
        assert_eq!(template.name, Some("PRD Template".to_string()));
    }

    #[test]
    fn methodology_template_with_description() {
        let template = MethodologyTemplate::new("prd", "templates/prd.md")
            .with_description("Product Requirements Document template");
        assert_eq!(template.description, Some("Product Requirements Document template".to_string()));
    }

    #[test]
    fn methodology_template_serializes() {
        let template = MethodologyTemplate::new("design_doc", "templates/design.md")
            .with_name("Design Doc")
            .with_description("Architecture design document");
        let json = serde_json::to_string(&template).unwrap();

        assert!(json.contains("\"artifact_type\":\"design_doc\""));
        assert!(json.contains("\"template_path\":\"templates/design.md\""));
        assert!(json.contains("\"name\":\"Design Doc\""));
        assert!(json.contains("\"description\":\"Architecture design document\""));
    }

    #[test]
    fn methodology_template_deserializes() {
        let json = r#"{"artifact_type":"prd","template_path":"prd.md"}"#;
        let template: MethodologyTemplate = serde_json::from_str(json).unwrap();

        assert_eq!(template.artifact_type, "prd");
        assert_eq!(template.template_path, "prd.md");
    }

    #[test]
    fn methodology_template_skips_optional_fields() {
        let template = MethodologyTemplate::new("prd", "prd.md");
        let json = serde_json::to_string(&template).unwrap();

        assert!(!json.contains("\"name\""));
        assert!(!json.contains("\"description\""));
    }

    // ===== MethodologyStatus Tests =====

    #[test]
    fn methodology_status_all_returns_3_statuses() {
        let all = MethodologyStatus::all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn methodology_status_as_str() {
        assert_eq!(MethodologyStatus::Available.as_str(), "available");
        assert_eq!(MethodologyStatus::Active.as_str(), "active");
        assert_eq!(MethodologyStatus::Disabled.as_str(), "disabled");
    }

    #[test]
    fn methodology_status_display() {
        assert_eq!(MethodologyStatus::Available.to_string(), "available");
        assert_eq!(MethodologyStatus::Active.to_string(), "active");
        assert_eq!(MethodologyStatus::Disabled.to_string(), "disabled");
    }

    #[test]
    fn methodology_status_from_str() {
        assert_eq!(MethodologyStatus::from_str("available").unwrap(), MethodologyStatus::Available);
        assert_eq!(MethodologyStatus::from_str("active").unwrap(), MethodologyStatus::Active);
        assert_eq!(MethodologyStatus::from_str("disabled").unwrap(), MethodologyStatus::Disabled);
    }

    #[test]
    fn methodology_status_from_str_error() {
        let err = MethodologyStatus::from_str("invalid").unwrap_err();
        assert_eq!(err.value, "invalid");
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn methodology_status_serializes() {
        assert_eq!(serde_json::to_string(&MethodologyStatus::Available).unwrap(), "\"available\"");
        assert_eq!(serde_json::to_string(&MethodologyStatus::Active).unwrap(), "\"active\"");
        assert_eq!(serde_json::to_string(&MethodologyStatus::Disabled).unwrap(), "\"disabled\"");
    }

    #[test]
    fn methodology_status_deserializes() {
        let s: MethodologyStatus = serde_json::from_str("\"active\"").unwrap();
        assert_eq!(s, MethodologyStatus::Active);
    }

    #[test]
    fn methodology_status_default_is_available() {
        assert_eq!(MethodologyStatus::default(), MethodologyStatus::Available);
    }
}
