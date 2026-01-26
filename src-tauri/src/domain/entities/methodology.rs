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

/// Configuration for plan artifacts in ideation flow
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodologyPlanArtifactConfig {
    /// Artifact type to use for plans (e.g., "specification", "design_doc")
    pub artifact_type: String,
    /// Bucket ID to store plans in (e.g., "prd-library")
    pub bucket_id: String,
}

/// Plan template provided by a methodology
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodologyPlanTemplate {
    /// Unique identifier for the template
    pub id: String,
    /// Display name for the template
    pub name: String,
    /// Description of when to use this template
    pub description: String,
    /// Markdown template content with {{placeholders}}
    pub template_content: String,
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
    /// Custom artifact configuration for ideation plans
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_artifact_config: Option<MethodologyPlanArtifactConfig>,
    /// Plan templates provided by this methodology
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_templates: Vec<MethodologyPlanTemplate>,
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
            plan_artifact_config: None,
            plan_templates: vec![],
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

    /// Sets the plan artifact configuration
    pub fn with_plan_artifact_config(mut self, config: MethodologyPlanArtifactConfig) -> Self {
        self.plan_artifact_config = Some(config);
        self
    }

    /// Adds a plan template
    pub fn with_plan_template(mut self, template: MethodologyPlanTemplate) -> Self {
        self.plan_templates.push(template);
        self
    }

    /// Adds multiple plan templates
    pub fn with_plan_templates(mut self, templates: impl IntoIterator<Item = MethodologyPlanTemplate>) -> Self {
        self.plan_templates.extend(templates);
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

    /// Returns the built-in BMAD methodology
    ///
    /// BMAD (Breakthrough Method for Agile AI-Driven Development) uses:
    /// - 8 agents: Analyst, PM, Architect, UX Designer, Developer, Scrum Master, TEA, Tech Writer
    /// - 4 phases: Analysis → Planning → Solutioning → Implementation
    /// - Document-centric: PRD, Architecture Doc, UX Design, Stories/Epics
    pub fn bmad() -> Self {
        use super::status::InternalStatus;
        use super::workflow::{ColumnBehavior, WorkflowColumn, WorkflowId, WorkflowSchema};

        let workflow = WorkflowSchema {
            id: WorkflowId::from_string("bmad-method"),
            name: "BMAD Method".to_string(),
            description: Some("Breakthrough Method for Agile AI-Driven Development".to_string()),
            columns: vec![
                // Phase 1: Analysis
                WorkflowColumn::new("brainstorm", "Brainstorm", InternalStatus::Backlog)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("bmad-analyst")),
                WorkflowColumn::new("research", "Research", InternalStatus::Executing)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("bmad-analyst")),
                // Phase 2: Planning
                WorkflowColumn::new("prd-draft", "PRD Draft", InternalStatus::Executing)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("bmad-pm")),
                WorkflowColumn::new("prd-review", "PRD Review", InternalStatus::PendingReview)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("bmad-pm")),
                WorkflowColumn::new("ux-design", "UX Design", InternalStatus::Executing)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("bmad-ux")),
                // Phase 3: Solutioning
                WorkflowColumn::new("architecture", "Architecture", InternalStatus::Executing)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("bmad-architect")),
                WorkflowColumn::new("stories", "Stories", InternalStatus::Ready)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("bmad-pm")),
                // Phase 4: Implementation
                WorkflowColumn::new("sprint", "Sprint", InternalStatus::Executing)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("bmad-developer")),
                WorkflowColumn::new("code-review", "Code Review", InternalStatus::PendingReview)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("bmad-developer")),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
            external_sync: None,
            defaults: Default::default(),
            is_default: false,
        };

        Self {
            id: MethodologyId::from_string("bmad-method"),
            name: "BMAD Method".to_string(),
            description: Some(
                "Breakthrough Method for Agile AI-Driven Development - a document-centric \
                 methodology with 4 phases: Analysis, Planning, Solutioning, Implementation"
                    .to_string(),
            ),
            agent_profiles: vec![
                "bmad-analyst".to_string(),
                "bmad-pm".to_string(),
                "bmad-architect".to_string(),
                "bmad-ux".to_string(),
                "bmad-developer".to_string(),
                "bmad-scrum-master".to_string(),
                "bmad-tea".to_string(),
                "bmad-tech-writer".to_string(),
            ],
            skills: vec![
                "skills/prd-creation".to_string(),
                "skills/architecture-design".to_string(),
                "skills/ux-review".to_string(),
                "skills/story-writing".to_string(),
            ],
            workflow,
            phases: vec![
                MethodologyPhase::new("analysis", "Analysis", 0)
                    .with_description("Analyze requirements and research domain")
                    .with_agent_profiles(["bmad-analyst"])
                    .with_columns(["brainstorm", "research"]),
                MethodologyPhase::new("planning", "Planning", 1)
                    .with_description("Create PRD and UX design documents")
                    .with_agent_profiles(["bmad-pm", "bmad-ux"])
                    .with_columns(["prd-draft", "prd-review", "ux-design"]),
                MethodologyPhase::new("solutioning", "Solutioning", 2)
                    .with_description("Design architecture and create user stories")
                    .with_agent_profiles(["bmad-architect", "bmad-pm"])
                    .with_columns(["architecture", "stories"]),
                MethodologyPhase::new("implementation", "Implementation", 3)
                    .with_description("Execute sprints and code review")
                    .with_agent_profiles(["bmad-developer"])
                    .with_columns(["sprint", "code-review", "done"]),
            ],
            templates: vec![
                MethodologyTemplate::new("prd", "templates/bmad/prd.md")
                    .with_name("PRD Template")
                    .with_description("Product Requirements Document for BMAD"),
                MethodologyTemplate::new("design_doc", "templates/bmad/architecture.md")
                    .with_name("Architecture Document")
                    .with_description("System architecture design document"),
                MethodologyTemplate::new("specification", "templates/bmad/ux-design.md")
                    .with_name("UX Design Spec")
                    .with_description("User experience design specification"),
            ],
            plan_artifact_config: None,
            plan_templates: vec![],
            hooks_config: Some(serde_json::json!({
                "phase_gates": {
                    "analysis": ["requirements_documented"],
                    "planning": ["prd_approved", "ux_approved"],
                    "solutioning": ["architecture_approved"]
                },
                "validation_checklists": {
                    "prd": ["clear_objectives", "success_metrics", "scope_defined"],
                    "architecture": ["scalability", "security", "maintainability"]
                }
            })),
            is_active: false,
            created_at: Utc::now(),
        }
    }

    /// Returns the built-in GSD methodology
    ///
    /// GSD (Get Shit Done) uses:
    /// - 11 agents: project-researcher, phase-researcher, planner, executor, verifier, debugger, etc.
    /// - Wave-based parallelization: Plans grouped into waves for parallel execution
    /// - Checkpoint protocol: human-verify, decision, human-action types
    /// - Goal-backward verification: must-haves derived from phase goals
    pub fn gsd() -> Self {
        use super::status::InternalStatus;
        use super::workflow::{ColumnBehavior, WorkflowColumn, WorkflowId, WorkflowSchema};

        let workflow = WorkflowSchema {
            id: WorkflowId::from_string("gsd-method"),
            name: "GSD (Get Shit Done)".to_string(),
            description: Some(
                "Spec-driven development with wave-based parallelization".to_string(),
            ),
            columns: vec![
                // Initialize
                WorkflowColumn::new("initialize", "Initialize", InternalStatus::Backlog)
                    .with_behavior(
                        ColumnBehavior::new().with_agent_profile("gsd-project-researcher"),
                    ),
                // Discuss (optional)
                WorkflowColumn::new("discuss", "Discuss", InternalStatus::Blocked)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("gsd-orchestrator")),
                // Plan
                WorkflowColumn::new("research", "Research", InternalStatus::Executing)
                    .with_behavior(
                        ColumnBehavior::new().with_agent_profile("gsd-phase-researcher"),
                    ),
                WorkflowColumn::new("planning", "Planning", InternalStatus::Executing)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("gsd-planner")),
                WorkflowColumn::new("plan-check", "Plan Check", InternalStatus::PendingReview)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("gsd-plan-checker")),
                // Execute (wave-based)
                WorkflowColumn::new("queued", "Queued", InternalStatus::Ready),
                WorkflowColumn::new("executing", "Executing", InternalStatus::Executing)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("gsd-executor")),
                WorkflowColumn::new("checkpoint", "Checkpoint", InternalStatus::Blocked),
                // Verify
                WorkflowColumn::new("verifying", "Verifying", InternalStatus::PendingReview)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("gsd-verifier")),
                WorkflowColumn::new("debugging", "Debugging", InternalStatus::RevisionNeeded)
                    .with_behavior(ColumnBehavior::new().with_agent_profile("gsd-debugger")),
                // Complete
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
            external_sync: None,
            defaults: Default::default(),
            is_default: false,
        };

        Self {
            id: MethodologyId::from_string("gsd-method"),
            name: "GSD (Get Shit Done)".to_string(),
            description: Some(
                "Spec-driven development with wave-based parallelization. Features checkpoint \
                 protocols (human-verify, decision, human-action) and goal-backward verification \
                 with must-haves derived from phase goals."
                    .to_string(),
            ),
            agent_profiles: vec![
                "gsd-project-researcher".to_string(),
                "gsd-phase-researcher".to_string(),
                "gsd-planner".to_string(),
                "gsd-plan-checker".to_string(),
                "gsd-executor".to_string(),
                "gsd-verifier".to_string(),
                "gsd-debugger".to_string(),
                "gsd-orchestrator".to_string(),
                "gsd-monitor".to_string(),
                "gsd-qa".to_string(),
                "gsd-docs".to_string(),
            ],
            skills: vec![
                "skills/project-analysis".to_string(),
                "skills/phase-research".to_string(),
                "skills/wave-planning".to_string(),
                "skills/checkpoint-handling".to_string(),
                "skills/verification".to_string(),
            ],
            workflow,
            phases: vec![
                MethodologyPhase::new("initialize", "Initialize", 0)
                    .with_description("Project research and initialization")
                    .with_agent_profiles(["gsd-project-researcher"])
                    .with_column("initialize"),
                MethodologyPhase::new("plan", "Plan", 1)
                    .with_description("Research, planning, and plan verification")
                    .with_agent_profiles(["gsd-phase-researcher", "gsd-planner", "gsd-plan-checker"])
                    .with_columns(["discuss", "research", "planning", "plan-check"]),
                MethodologyPhase::new("execute", "Execute", 2)
                    .with_description("Wave-based parallel execution with checkpoints")
                    .with_agent_profiles(["gsd-executor"])
                    .with_columns(["queued", "executing", "checkpoint"]),
                MethodologyPhase::new("verify", "Verify", 3)
                    .with_description("Verification and debugging")
                    .with_agent_profiles(["gsd-verifier", "gsd-debugger"])
                    .with_columns(["verifying", "debugging", "done"]),
            ],
            templates: vec![
                MethodologyTemplate::new("specification", "templates/gsd/phase-spec.md")
                    .with_name("Phase Specification")
                    .with_description("Specification for a GSD phase"),
                MethodologyTemplate::new("task_spec", "templates/gsd/plan-spec.md")
                    .with_name("Plan Specification")
                    .with_description("Detailed plan specification with must-haves"),
                MethodologyTemplate::new("context", "templates/gsd/state.md")
                    .with_name("STATE.md Template")
                    .with_description("State tracking document for GSD execution"),
            ],
            plan_artifact_config: None,
            plan_templates: vec![],
            hooks_config: Some(serde_json::json!({
                "checkpoint_types": ["auto", "human-verify", "decision", "human-action"],
                "wave_execution": {
                    "max_parallel": 5,
                    "wave_completion_required": true
                },
                "verification": {
                    "must_haves_required": true,
                    "goal_backward_check": true
                }
            })),
            is_active: false,
            created_at: Utc::now(),
        }
    }

    /// Returns all built-in methodologies
    pub fn builtin_methodologies() -> Vec<Self> {
        vec![Self::bmad(), Self::gsd()]
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

    // ===== Built-in Methodology Tests =====

    #[test]
    fn bmad_methodology_has_correct_id() {
        let bmad = MethodologyExtension::bmad();
        assert_eq!(bmad.id.as_str(), "bmad-method");
    }

    #[test]
    fn bmad_methodology_has_correct_name() {
        let bmad = MethodologyExtension::bmad();
        assert_eq!(bmad.name, "BMAD Method");
    }

    #[test]
    fn bmad_methodology_has_8_agent_profiles() {
        let bmad = MethodologyExtension::bmad();
        assert_eq!(bmad.agent_profiles.len(), 8);
        assert!(bmad.agent_profiles.contains(&"bmad-analyst".to_string()));
        assert!(bmad.agent_profiles.contains(&"bmad-pm".to_string()));
        assert!(bmad.agent_profiles.contains(&"bmad-architect".to_string()));
        assert!(bmad.agent_profiles.contains(&"bmad-ux".to_string()));
        assert!(bmad.agent_profiles.contains(&"bmad-developer".to_string()));
        assert!(bmad.agent_profiles.contains(&"bmad-scrum-master".to_string()));
        assert!(bmad.agent_profiles.contains(&"bmad-tea".to_string()));
        assert!(bmad.agent_profiles.contains(&"bmad-tech-writer".to_string()));
    }

    #[test]
    fn bmad_methodology_has_4_phases() {
        let bmad = MethodologyExtension::bmad();
        assert_eq!(bmad.phases.len(), 4);

        let sorted = bmad.sorted_phases();
        assert_eq!(sorted[0].name, "Analysis");
        assert_eq!(sorted[1].name, "Planning");
        assert_eq!(sorted[2].name, "Solutioning");
        assert_eq!(sorted[3].name, "Implementation");
    }

    #[test]
    fn bmad_methodology_has_10_workflow_columns() {
        let bmad = MethodologyExtension::bmad();
        assert_eq!(bmad.workflow.columns.len(), 10);

        let column_ids: Vec<_> = bmad.workflow.columns.iter().map(|c| c.id.as_str()).collect();
        assert!(column_ids.contains(&"brainstorm"));
        assert!(column_ids.contains(&"research"));
        assert!(column_ids.contains(&"prd-draft"));
        assert!(column_ids.contains(&"prd-review"));
        assert!(column_ids.contains(&"ux-design"));
        assert!(column_ids.contains(&"architecture"));
        assert!(column_ids.contains(&"stories"));
        assert!(column_ids.contains(&"sprint"));
        assert!(column_ids.contains(&"code-review"));
        assert!(column_ids.contains(&"done"));
    }

    #[test]
    fn bmad_methodology_has_templates() {
        let bmad = MethodologyExtension::bmad();
        assert_eq!(bmad.templates.len(), 3);

        let template_types: Vec<_> = bmad.templates.iter().map(|t| t.artifact_type.as_str()).collect();
        assert!(template_types.contains(&"prd"));
        assert!(template_types.contains(&"design_doc"));
        assert!(template_types.contains(&"specification"));
    }

    #[test]
    fn bmad_methodology_has_hooks_config() {
        let bmad = MethodologyExtension::bmad();
        assert!(bmad.hooks_config.is_some());

        let hooks = bmad.hooks_config.unwrap();
        assert!(hooks.get("phase_gates").is_some());
        assert!(hooks.get("validation_checklists").is_some());
    }

    #[test]
    fn bmad_methodology_not_active_by_default() {
        let bmad = MethodologyExtension::bmad();
        assert!(!bmad.is_active);
    }

    #[test]
    fn gsd_methodology_has_correct_id() {
        let gsd = MethodologyExtension::gsd();
        assert_eq!(gsd.id.as_str(), "gsd-method");
    }

    #[test]
    fn gsd_methodology_has_correct_name() {
        let gsd = MethodologyExtension::gsd();
        assert_eq!(gsd.name, "GSD (Get Shit Done)");
    }

    #[test]
    fn gsd_methodology_has_11_agent_profiles() {
        let gsd = MethodologyExtension::gsd();
        assert_eq!(gsd.agent_profiles.len(), 11);
        assert!(gsd.agent_profiles.contains(&"gsd-project-researcher".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-phase-researcher".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-planner".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-plan-checker".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-executor".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-verifier".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-debugger".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-orchestrator".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-monitor".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-qa".to_string()));
        assert!(gsd.agent_profiles.contains(&"gsd-docs".to_string()));
    }

    #[test]
    fn gsd_methodology_has_4_phases() {
        let gsd = MethodologyExtension::gsd();
        assert_eq!(gsd.phases.len(), 4);

        let sorted = gsd.sorted_phases();
        assert_eq!(sorted[0].name, "Initialize");
        assert_eq!(sorted[1].name, "Plan");
        assert_eq!(sorted[2].name, "Execute");
        assert_eq!(sorted[3].name, "Verify");
    }

    #[test]
    fn gsd_methodology_has_11_workflow_columns() {
        let gsd = MethodologyExtension::gsd();
        assert_eq!(gsd.workflow.columns.len(), 11);

        let column_ids: Vec<_> = gsd.workflow.columns.iter().map(|c| c.id.as_str()).collect();
        assert!(column_ids.contains(&"initialize"));
        assert!(column_ids.contains(&"discuss"));
        assert!(column_ids.contains(&"research"));
        assert!(column_ids.contains(&"planning"));
        assert!(column_ids.contains(&"plan-check"));
        assert!(column_ids.contains(&"queued"));
        assert!(column_ids.contains(&"executing"));
        assert!(column_ids.contains(&"checkpoint"));
        assert!(column_ids.contains(&"verifying"));
        assert!(column_ids.contains(&"debugging"));
        assert!(column_ids.contains(&"done"));
    }

    #[test]
    fn gsd_methodology_has_templates() {
        let gsd = MethodologyExtension::gsd();
        assert_eq!(gsd.templates.len(), 3);

        let template_types: Vec<_> = gsd.templates.iter().map(|t| t.artifact_type.as_str()).collect();
        assert!(template_types.contains(&"specification"));
        assert!(template_types.contains(&"task_spec"));
        assert!(template_types.contains(&"context"));
    }

    #[test]
    fn gsd_methodology_has_hooks_config() {
        let gsd = MethodologyExtension::gsd();
        assert!(gsd.hooks_config.is_some());

        let hooks = gsd.hooks_config.unwrap();
        assert!(hooks.get("checkpoint_types").is_some());
        assert!(hooks.get("wave_execution").is_some());
        assert!(hooks.get("verification").is_some());
    }

    #[test]
    fn gsd_methodology_not_active_by_default() {
        let gsd = MethodologyExtension::gsd();
        assert!(!gsd.is_active);
    }

    #[test]
    fn builtin_methodologies_returns_two() {
        let methodologies = MethodologyExtension::builtin_methodologies();
        assert_eq!(methodologies.len(), 2);
    }

    #[test]
    fn builtin_methodologies_includes_bmad_and_gsd() {
        let methodologies = MethodologyExtension::builtin_methodologies();
        let names: Vec<_> = methodologies.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"BMAD Method"));
        assert!(names.contains(&"GSD (Get Shit Done)"));
    }

    #[test]
    fn bmad_workflow_column_behaviors_have_agent_profiles() {
        let bmad = MethodologyExtension::bmad();

        // Check that columns have correct agent behaviors
        let brainstorm = bmad.workflow.columns.iter().find(|c| c.id == "brainstorm").unwrap();
        assert!(brainstorm.behavior.is_some());
        assert_eq!(brainstorm.behavior.as_ref().unwrap().agent_profile, Some("bmad-analyst".to_string()));

        let sprint = bmad.workflow.columns.iter().find(|c| c.id == "sprint").unwrap();
        assert!(sprint.behavior.is_some());
        assert_eq!(sprint.behavior.as_ref().unwrap().agent_profile, Some("bmad-developer".to_string()));
    }

    #[test]
    fn gsd_workflow_column_behaviors_have_agent_profiles() {
        let gsd = MethodologyExtension::gsd();

        // Check that columns have correct agent behaviors
        let initialize = gsd.workflow.columns.iter().find(|c| c.id == "initialize").unwrap();
        assert!(initialize.behavior.is_some());
        assert_eq!(initialize.behavior.as_ref().unwrap().agent_profile, Some("gsd-project-researcher".to_string()));

        let executing = gsd.workflow.columns.iter().find(|c| c.id == "executing").unwrap();
        assert!(executing.behavior.is_some());
        assert_eq!(executing.behavior.as_ref().unwrap().agent_profile, Some("gsd-executor".to_string()));
    }

    #[test]
    fn bmad_methodology_serializes_roundtrip() {
        let bmad = MethodologyExtension::bmad();
        let json = serde_json::to_string(&bmad).unwrap();
        let parsed: MethodologyExtension = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id.as_str(), "bmad-method");
        assert_eq!(parsed.name, "BMAD Method");
        assert_eq!(parsed.agent_profiles.len(), 8);
        assert_eq!(parsed.phases.len(), 4);
        assert_eq!(parsed.workflow.columns.len(), 10);
    }

    #[test]
    fn gsd_methodology_serializes_roundtrip() {
        let gsd = MethodologyExtension::gsd();
        let json = serde_json::to_string(&gsd).unwrap();
        let parsed: MethodologyExtension = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id.as_str(), "gsd-method");
        assert_eq!(parsed.name, "GSD (Get Shit Done)");
        assert_eq!(parsed.agent_profiles.len(), 11);
        assert_eq!(parsed.phases.len(), 4);
        assert_eq!(parsed.workflow.columns.len(), 11);
    }
}
