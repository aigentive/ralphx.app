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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MethodologyStatus {
    /// Available but not active
    #[default]
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

#[cfg(test)]
mod tests;
