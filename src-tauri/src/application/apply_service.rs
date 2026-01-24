// ApplyService
// Application service for converting task proposals to real tasks
//
// This service handles the "apply" flow:
// - Validating selected proposals have no circular dependencies
// - Creating Task entities from TaskProposal entities
// - Copying proposal dependencies to task dependencies
// - Updating proposal status and linking to created tasks
// - Optionally marking the session as "converted"

use crate::domain::entities::{
    IdeationSessionId, IdeationSessionStatus, InternalStatus, ProjectId, ProposalStatus, Task,
    TaskId, TaskProposal, TaskProposalId,
};
use crate::domain::repositories::{
    IdeationSessionRepository, ProposalDependencyRepository, TaskDependencyRepository,
    TaskProposalRepository, TaskRepository,
};
use crate::error::{AppError, AppResult};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Target column for applied tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetColumn {
    /// Draft column - tasks need refinement
    Draft,
    /// Backlog column - confirmed but not scheduled
    Backlog,
    /// Todo/Ready column - ready for execution
    Todo,
}

impl TargetColumn {
    /// Convert to InternalStatus
    pub fn to_status(&self) -> InternalStatus {
        match self {
            TargetColumn::Draft => InternalStatus::Backlog,
            TargetColumn::Backlog => InternalStatus::Backlog,
            TargetColumn::Todo => InternalStatus::Ready,
        }
    }
}

/// Options for applying proposals to the Kanban
#[derive(Debug, Clone)]
pub struct ApplyProposalsOptions {
    /// IDs of proposals to apply
    pub proposal_ids: Vec<TaskProposalId>,
    /// Target column for created tasks
    pub target_column: TargetColumn,
    /// Whether to create task dependencies from proposal dependencies
    pub preserve_dependencies: bool,
}

/// Result of applying proposals
#[derive(Debug, Clone)]
pub struct ApplyProposalsResult {
    /// Tasks that were created
    pub created_tasks: Vec<Task>,
    /// Number of dependencies created
    pub dependencies_created: u32,
    /// Any warnings encountered
    pub warnings: Vec<String>,
    /// Whether the session was marked as converted
    pub session_converted: bool,
}

/// Validation result for selected proposals
#[derive(Debug, Clone)]
pub struct SelectionValidation {
    /// Whether the selection is valid
    pub is_valid: bool,
    /// Circular dependency cycles found (if any)
    pub cycles: Vec<Vec<TaskProposalId>>,
    /// Warning messages
    pub warnings: Vec<String>,
}

/// Service for converting proposals to tasks
pub struct ApplyService<
    S: IdeationSessionRepository,
    P: TaskProposalRepository,
    PD: ProposalDependencyRepository,
    T: TaskRepository,
    TD: TaskDependencyRepository,
> {
    session_repo: Arc<S>,
    proposal_repo: Arc<P>,
    proposal_dep_repo: Arc<PD>,
    task_repo: Arc<T>,
    task_dep_repo: Arc<TD>,
}

impl<S, P, PD, T, TD> ApplyService<S, P, PD, T, TD>
where
    S: IdeationSessionRepository,
    P: TaskProposalRepository,
    PD: ProposalDependencyRepository,
    T: TaskRepository,
    TD: TaskDependencyRepository,
{
    /// Create a new apply service
    pub fn new(
        session_repo: Arc<S>,
        proposal_repo: Arc<P>,
        proposal_dep_repo: Arc<PD>,
        task_repo: Arc<T>,
        task_dep_repo: Arc<TD>,
    ) -> Self {
        Self {
            session_repo,
            proposal_repo,
            proposal_dep_repo,
            task_repo,
            task_dep_repo,
        }
    }

    /// Validate that the selected proposals have no circular dependencies
    pub async fn validate_selection(
        &self,
        session_id: &IdeationSessionId,
        proposal_ids: &[TaskProposalId],
    ) -> AppResult<SelectionValidation> {
        if proposal_ids.is_empty() {
            return Ok(SelectionValidation {
                is_valid: true,
                cycles: Vec::new(),
                warnings: vec!["No proposals selected".to_string()],
            });
        }

        // Get all dependencies for the session
        let all_deps = self.proposal_dep_repo.get_all_for_session(session_id).await?;

        // Build a set of selected proposal IDs for quick lookup
        let selected_set: HashSet<_> = proposal_ids.iter().map(|id| id.to_string()).collect();

        // Filter dependencies to only those between selected proposals
        let relevant_deps: Vec<_> = all_deps
            .iter()
            .filter(|(from, to)| {
                selected_set.contains(&from.to_string()) && selected_set.contains(&to.to_string())
            })
            .collect();

        // Build adjacency list for cycle detection
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for (from, to) in &relevant_deps {
            adj.entry(from.to_string())
                .or_default()
                .push(to.to_string());
        }

        // Detect cycles using DFS
        let cycles = self.detect_cycles(&selected_set, &adj);

        let mut warnings = Vec::new();

        // Check for missing dependencies (deps outside selection)
        for (from, to) in &all_deps {
            if selected_set.contains(&from.to_string()) && !selected_set.contains(&to.to_string()) {
                warnings.push(format!(
                    "Proposal {} depends on {} which is not selected",
                    from, to
                ));
            }
        }

        Ok(SelectionValidation {
            is_valid: cycles.is_empty(),
            cycles,
            warnings,
        })
    }

    /// Detect cycles in the dependency graph
    fn detect_cycles(
        &self,
        nodes: &HashSet<String>,
        adj: &HashMap<String, Vec<String>>,
    ) -> Vec<Vec<TaskProposalId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in nodes {
            if !visited.contains(node) {
                self.dfs_cycle_detect(node, adj, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    fn dfs_cycle_detect(
        &self,
        node: &str,
        adj: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<TaskProposalId>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = adj.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_cycle_detect(neighbor, adj, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle - extract it from path
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                    let cycle: Vec<TaskProposalId> = path[cycle_start..]
                        .iter()
                        .map(|id| TaskProposalId::from_string(id.clone()))
                        .collect();
                    if !cycle.is_empty() {
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
    }

    /// Apply selected proposals to the Kanban board, creating real tasks
    pub async fn apply_proposals(
        &self,
        session_id: &IdeationSessionId,
        options: ApplyProposalsOptions,
    ) -> AppResult<ApplyProposalsResult> {
        // Get the session to know the project_id
        let session = self
            .session_repo
            .get_by_id(session_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Session {} not found", session_id)))?;

        if session.status != IdeationSessionStatus::Active {
            return Err(AppError::Validation(format!(
                "Cannot apply proposals from {} session",
                session.status
            )));
        }

        // Validate selection
        let validation = self
            .validate_selection(session_id, &options.proposal_ids)
            .await?;

        if !validation.is_valid {
            return Err(AppError::Validation(format!(
                "Selection has circular dependencies: {:?}",
                validation.cycles
            )));
        }

        // Get all selected proposals
        let mut proposals_map: HashMap<String, TaskProposal> = HashMap::new();
        for proposal_id in &options.proposal_ids {
            if let Some(proposal) = self.proposal_repo.get_by_id(proposal_id).await? {
                proposals_map.insert(proposal_id.to_string(), proposal);
            }
        }

        if proposals_map.is_empty() {
            return Ok(ApplyProposalsResult {
                created_tasks: Vec::new(),
                dependencies_created: 0,
                warnings: vec!["No valid proposals found".to_string()],
                session_converted: false,
            });
        }

        // Create tasks and track proposal->task mapping
        let mut proposal_to_task: HashMap<String, TaskId> = HashMap::new();
        let mut created_tasks = Vec::new();
        let target_status = options.target_column.to_status();

        for proposal in proposals_map.values() {
            let task = self.create_task_from_proposal(proposal, &session.project_id, target_status);
            let created_task = self.task_repo.create(task).await?;

            // Update proposal with created task ID and status
            let mut updated_proposal = proposal.clone();
            updated_proposal.created_task_id = Some(created_task.id.clone());
            updated_proposal.status = ProposalStatus::Accepted;
            updated_proposal.touch();
            self.proposal_repo.update(&updated_proposal).await?;

            proposal_to_task.insert(proposal.id.to_string(), created_task.id.clone());
            created_tasks.push(created_task);
        }

        // Create task dependencies if requested
        let mut dependencies_created = 0;
        if options.preserve_dependencies {
            let all_deps = self.proposal_dep_repo.get_all_for_session(session_id).await?;

            for (from_proposal, to_proposal) in all_deps {
                // Only create dependency if both proposals were converted
                if let (Some(from_task), Some(to_task)) = (
                    proposal_to_task.get(&from_proposal.to_string()),
                    proposal_to_task.get(&to_proposal.to_string()),
                ) {
                    self.task_dep_repo
                        .add_dependency(from_task, to_task)
                        .await?;
                    dependencies_created += 1;
                }
            }
        }

        // Check if all proposals in session are now converted
        let session_converted = self.check_and_update_session_status(session_id).await?;

        Ok(ApplyProposalsResult {
            created_tasks,
            dependencies_created,
            warnings: validation.warnings,
            session_converted,
        })
    }

    /// Create a Task from a TaskProposal
    fn create_task_from_proposal(
        &self,
        proposal: &TaskProposal,
        project_id: &ProjectId,
        status: InternalStatus,
    ) -> Task {
        let mut task = Task::new_with_category(
            project_id.clone(),
            proposal.title.clone(),
            proposal.category.to_string(),
        );

        task.description = proposal.description.clone();
        task.priority = proposal.priority_score;
        task.internal_status = status;

        task
    }

    /// Check if session should be marked as converted and update if so
    async fn check_and_update_session_status(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<bool> {
        // Get all proposals in the session
        let proposals = self.proposal_repo.get_by_session(session_id).await?;

        // Check if all proposals have been converted (have a created_task_id)
        let all_converted = proposals.iter().all(|p| p.created_task_id.is_some());

        if all_converted && !proposals.is_empty() {
            self.session_repo
                .update_status(session_id, IdeationSessionStatus::Converted)
                .await?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Apply all selected proposals from a session
    pub async fn apply_selected_proposals(
        &self,
        session_id: &IdeationSessionId,
        target_column: TargetColumn,
        preserve_dependencies: bool,
    ) -> AppResult<ApplyProposalsResult> {
        // Get all selected proposals
        let selected = self
            .proposal_repo
            .get_selected_by_session(session_id)
            .await?;

        let proposal_ids: Vec<TaskProposalId> = selected.iter().map(|p| p.id.clone()).collect();

        self.apply_proposals(
            session_id,
            ApplyProposalsOptions {
                proposal_ids,
                target_column,
                preserve_dependencies,
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::domain::entities::{
        IdeationSession, Priority, PriorityAssessment, TaskCategory,
    };
    use std::sync::Mutex;

    // ========================================================================
    // MOCK REPOSITORIES
    // ========================================================================

    struct MockSessionRepository {
        sessions: Mutex<HashMap<String, IdeationSession>>,
    }

    impl MockSessionRepository {
        fn new() -> Self {
            Self {
                sessions: Mutex::new(HashMap::new()),
            }
        }

        fn with_session(session: IdeationSession) -> Self {
            let repo = Self::new();
            repo.sessions
                .lock()
                .unwrap()
                .insert(session.id.to_string(), session);
            repo
        }
    }

    #[async_trait]
    impl IdeationSessionRepository for MockSessionRepository {
        async fn create(&self, session: IdeationSession) -> AppResult<IdeationSession> {
            self.sessions
                .lock()
                .unwrap()
                .insert(session.id.to_string(), session.clone());
            Ok(session)
        }

        async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
            Ok(self.sessions.lock().unwrap().get(&id.to_string()).cloned())
        }

        async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
            Ok(self
                .sessions
                .lock()
                .unwrap()
                .values()
                .filter(|s| &s.project_id == project_id)
                .cloned()
                .collect())
        }

        async fn update_status(
            &self,
            id: &IdeationSessionId,
            status: IdeationSessionStatus,
        ) -> AppResult<()> {
            if let Some(session) = self.sessions.lock().unwrap().get_mut(&id.to_string()) {
                session.status = status;
            }
            Ok(())
        }

        async fn update_title(&self, id: &IdeationSessionId, title: Option<String>) -> AppResult<()> {
            if let Some(session) = self.sessions.lock().unwrap().get_mut(&id.to_string()) {
                session.title = title;
            }
            Ok(())
        }

        async fn delete(&self, id: &IdeationSessionId) -> AppResult<()> {
            self.sessions.lock().unwrap().remove(&id.to_string());
            Ok(())
        }

        async fn get_active_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
            Ok(self
                .sessions
                .lock()
                .unwrap()
                .values()
                .filter(|s| &s.project_id == project_id && s.status == IdeationSessionStatus::Active)
                .cloned()
                .collect())
        }

        async fn count_by_status(
            &self,
            project_id: &ProjectId,
            status: IdeationSessionStatus,
        ) -> AppResult<u32> {
            Ok(self
                .sessions
                .lock()
                .unwrap()
                .values()
                .filter(|s| &s.project_id == project_id && s.status == status)
                .count() as u32)
        }
    }

    struct MockProposalRepository {
        proposals: Mutex<HashMap<String, TaskProposal>>,
    }

    impl MockProposalRepository {
        fn new() -> Self {
            Self {
                proposals: Mutex::new(HashMap::new()),
            }
        }

        fn with_proposals(proposals: Vec<TaskProposal>) -> Self {
            let repo = Self::new();
            for p in proposals {
                repo.proposals
                    .lock()
                    .unwrap()
                    .insert(p.id.to_string(), p);
            }
            repo
        }
    }

    #[async_trait]
    impl TaskProposalRepository for MockProposalRepository {
        async fn create(&self, proposal: TaskProposal) -> AppResult<TaskProposal> {
            self.proposals
                .lock()
                .unwrap()
                .insert(proposal.id.to_string(), proposal.clone());
            Ok(proposal)
        }

        async fn get_by_id(&self, id: &TaskProposalId) -> AppResult<Option<TaskProposal>> {
            Ok(self.proposals.lock().unwrap().get(&id.to_string()).cloned())
        }

        async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<TaskProposal>> {
            let mut proposals: Vec<_> = self
                .proposals
                .lock()
                .unwrap()
                .values()
                .filter(|p| &p.session_id == session_id)
                .cloned()
                .collect();
            proposals.sort_by_key(|p| p.sort_order);
            Ok(proposals)
        }

        async fn update(&self, proposal: &TaskProposal) -> AppResult<()> {
            self.proposals
                .lock()
                .unwrap()
                .insert(proposal.id.to_string(), proposal.clone());
            Ok(())
        }

        async fn update_priority(
            &self,
            id: &TaskProposalId,
            assessment: &PriorityAssessment,
        ) -> AppResult<()> {
            if let Some(p) = self.proposals.lock().unwrap().get_mut(&id.to_string()) {
                p.suggested_priority = assessment.suggested_priority;
                p.priority_score = assessment.priority_score;
            }
            Ok(())
        }

        async fn update_selection(&self, id: &TaskProposalId, selected: bool) -> AppResult<()> {
            if let Some(p) = self.proposals.lock().unwrap().get_mut(&id.to_string()) {
                p.selected = selected;
            }
            Ok(())
        }

        async fn set_created_task_id(&self, id: &TaskProposalId, task_id: &TaskId) -> AppResult<()> {
            if let Some(p) = self.proposals.lock().unwrap().get_mut(&id.to_string()) {
                p.created_task_id = Some(task_id.clone());
            }
            Ok(())
        }

        async fn delete(&self, id: &TaskProposalId) -> AppResult<()> {
            self.proposals.lock().unwrap().remove(&id.to_string());
            Ok(())
        }

        async fn reorder(
            &self,
            _session_id: &IdeationSessionId,
            proposal_ids: Vec<TaskProposalId>,
        ) -> AppResult<()> {
            for (i, id) in proposal_ids.iter().enumerate() {
                if let Some(p) = self.proposals.lock().unwrap().get_mut(&id.to_string()) {
                    p.sort_order = i as i32;
                }
            }
            Ok(())
        }

        async fn get_selected_by_session(
            &self,
            session_id: &IdeationSessionId,
        ) -> AppResult<Vec<TaskProposal>> {
            let mut proposals: Vec<_> = self
                .proposals
                .lock()
                .unwrap()
                .values()
                .filter(|p| &p.session_id == session_id && p.selected)
                .cloned()
                .collect();
            proposals.sort_by_key(|p| p.sort_order);
            Ok(proposals)
        }

        async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .values()
                .filter(|p| &p.session_id == session_id)
                .count() as u32)
        }

        async fn count_selected_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .values()
                .filter(|p| &p.session_id == session_id && p.selected)
                .count() as u32)
        }
    }

    struct MockProposalDependencyRepository {
        dependencies: Mutex<Vec<(String, String)>>,
    }

    impl MockProposalDependencyRepository {
        fn new() -> Self {
            Self {
                dependencies: Mutex::new(Vec::new()),
            }
        }

        fn with_dependencies(deps: Vec<(TaskProposalId, TaskProposalId)>) -> Self {
            let repo = Self::new();
            for (from, to) in deps {
                repo.dependencies
                    .lock()
                    .unwrap()
                    .push((from.to_string(), to.to_string()));
            }
            repo
        }
    }

    #[async_trait]
    impl ProposalDependencyRepository for MockProposalDependencyRepository {
        async fn add_dependency(
            &self,
            proposal_id: &TaskProposalId,
            depends_on_id: &TaskProposalId,
        ) -> AppResult<()> {
            self.dependencies
                .lock()
                .unwrap()
                .push((proposal_id.to_string(), depends_on_id.to_string()));
            Ok(())
        }

        async fn remove_dependency(
            &self,
            proposal_id: &TaskProposalId,
            depends_on_id: &TaskProposalId,
        ) -> AppResult<()> {
            self.dependencies.lock().unwrap().retain(|(p, d)| {
                p != &proposal_id.to_string() || d != &depends_on_id.to_string()
            });
            Ok(())
        }

        async fn get_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<Vec<TaskProposalId>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(p, _)| p == &proposal_id.to_string())
                .map(|(_, d)| TaskProposalId::from_string(d.clone()))
                .collect())
        }

        async fn get_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<Vec<TaskProposalId>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, d)| d == &proposal_id.to_string())
                .map(|(p, _)| TaskProposalId::from_string(p.clone()))
                .collect())
        }

        async fn get_all_for_session(
            &self,
            _session_id: &IdeationSessionId,
        ) -> AppResult<Vec<(TaskProposalId, TaskProposalId)>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .map(|(p, d)| {
                    (
                        TaskProposalId::from_string(p.clone()),
                        TaskProposalId::from_string(d.clone()),
                    )
                })
                .collect())
        }

        async fn would_create_cycle(
            &self,
            _proposal_id: &TaskProposalId,
            _depends_on_id: &TaskProposalId,
        ) -> AppResult<bool> {
            Ok(false)
        }

        async fn clear_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<()> {
            self.dependencies.lock().unwrap().retain(|(p, d)| {
                p != &proposal_id.to_string() && d != &proposal_id.to_string()
            });
            Ok(())
        }

        async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(p, _)| p == &proposal_id.to_string())
                .count() as u32)
        }

        async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, d)| d == &proposal_id.to_string())
                .count() as u32)
        }
    }

    struct MockTaskRepository {
        tasks: Mutex<HashMap<String, Task>>,
    }

    impl MockTaskRepository {
        fn new() -> Self {
            Self {
                tasks: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl TaskRepository for MockTaskRepository {
        async fn create(&self, task: Task) -> AppResult<Task> {
            self.tasks
                .lock()
                .unwrap()
                .insert(task.id.to_string(), task.clone());
            Ok(task)
        }

        async fn get_by_id(&self, id: &TaskId) -> AppResult<Option<Task>> {
            Ok(self.tasks.lock().unwrap().get(&id.to_string()).cloned())
        }

        async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<Task>> {
            Ok(self
                .tasks
                .lock()
                .unwrap()
                .values()
                .filter(|t| &t.project_id == project_id)
                .cloned()
                .collect())
        }

        async fn update(&self, task: &Task) -> AppResult<()> {
            self.tasks
                .lock()
                .unwrap()
                .insert(task.id.to_string(), task.clone());
            Ok(())
        }

        async fn delete(&self, id: &TaskId) -> AppResult<()> {
            self.tasks.lock().unwrap().remove(&id.to_string());
            Ok(())
        }

        async fn get_by_status(&self, project_id: &ProjectId, status: InternalStatus) -> AppResult<Vec<Task>> {
            Ok(self
                .tasks
                .lock()
                .unwrap()
                .values()
                .filter(|t| &t.project_id == project_id && t.internal_status == status)
                .cloned()
                .collect())
        }

        async fn persist_status_change(
            &self,
            _id: &TaskId,
            _from: InternalStatus,
            _to: InternalStatus,
            _trigger: &str,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn get_status_history(&self, _id: &TaskId) -> AppResult<Vec<crate::domain::repositories::StatusTransition>> {
            Ok(vec![])
        }

        async fn get_next_executable(&self, _project_id: &ProjectId) -> AppResult<Option<Task>> {
            Ok(None)
        }

        async fn get_blockers(&self, _id: &TaskId) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn get_dependents(&self, _id: &TaskId) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn add_blocker(&self, _task_id: &TaskId, _blocker_id: &TaskId) -> AppResult<()> {
            Ok(())
        }

        async fn resolve_blocker(&self, _task_id: &TaskId, _blocker_id: &TaskId) -> AppResult<()> {
            Ok(())
        }
    }

    struct MockTaskDependencyRepository {
        dependencies: Mutex<Vec<(String, String)>>,
    }

    impl MockTaskDependencyRepository {
        fn new() -> Self {
            Self {
                dependencies: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl TaskDependencyRepository for MockTaskDependencyRepository {
        async fn add_dependency(
            &self,
            task_id: &TaskId,
            depends_on_task_id: &TaskId,
        ) -> AppResult<()> {
            self.dependencies
                .lock()
                .unwrap()
                .push((task_id.to_string(), depends_on_task_id.to_string()));
            Ok(())
        }

        async fn remove_dependency(
            &self,
            task_id: &TaskId,
            depends_on_task_id: &TaskId,
        ) -> AppResult<()> {
            self.dependencies.lock().unwrap().retain(|(t, d)| {
                t != &task_id.to_string() || d != &depends_on_task_id.to_string()
            });
            Ok(())
        }

        async fn get_blockers(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(t, _)| t == &task_id.to_string())
                .map(|(_, d)| TaskId::from_string(d.clone()))
                .collect())
        }

        async fn get_blocked_by(&self, task_id: &TaskId) -> AppResult<Vec<TaskId>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, d)| d == &task_id.to_string())
                .map(|(t, _)| TaskId::from_string(t.clone()))
                .collect())
        }

        async fn has_circular_dependency(
            &self,
            _task_id: &TaskId,
            _potential_dep: &TaskId,
        ) -> AppResult<bool> {
            Ok(false)
        }

        async fn clear_dependencies(&self, task_id: &TaskId) -> AppResult<()> {
            self.dependencies.lock().unwrap().retain(|(t, d)| {
                t != &task_id.to_string() && d != &task_id.to_string()
            });
            Ok(())
        }

        async fn count_blockers(&self, task_id: &TaskId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(t, _)| t == &task_id.to_string())
                .count() as u32)
        }

        async fn count_blocked_by(&self, task_id: &TaskId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, d)| d == &task_id.to_string())
                .count() as u32)
        }

        async fn has_dependency(&self, task_id: &TaskId, depends_on_task_id: &TaskId) -> AppResult<bool> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .any(|(t, d)| t == &task_id.to_string() && d == &depends_on_task_id.to_string()))
        }
    }

    // ========================================================================
    // HELPER FUNCTIONS
    // ========================================================================

    fn create_service(
        session: IdeationSession,
        proposals: Vec<TaskProposal>,
        deps: Vec<(TaskProposalId, TaskProposalId)>,
    ) -> ApplyService<
        MockSessionRepository,
        MockProposalRepository,
        MockProposalDependencyRepository,
        MockTaskRepository,
        MockTaskDependencyRepository,
    > {
        ApplyService::new(
            Arc::new(MockSessionRepository::with_session(session)),
            Arc::new(MockProposalRepository::with_proposals(proposals)),
            Arc::new(MockProposalDependencyRepository::with_dependencies(deps)),
            Arc::new(MockTaskRepository::new()),
            Arc::new(MockTaskDependencyRepository::new()),
        )
    }

    fn create_test_proposal(session_id: &IdeationSessionId, title: &str) -> TaskProposal {
        TaskProposal::new(
            session_id.clone(),
            title,
            TaskCategory::Feature,
            Priority::Medium,
        )
    }

    // ========================================================================
    // VALIDATION TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_validate_selection_empty() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let service = create_service(session.clone(), vec![], vec![]);

        let result = service
            .validate_selection(&session.id, &[])
            .await
            .unwrap();

        assert!(result.is_valid);
        assert!(result.cycles.is_empty());
        assert!(!result.warnings.is_empty());
    }

    #[tokio::test]
    async fn test_validate_selection_no_cycles() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "Setup");
        let p2 = create_test_proposal(&session_id, "Feature");
        let p3 = create_test_proposal(&session_id, "Tests");

        // p2 depends on p1, p3 depends on p2 (linear chain)
        let deps = vec![
            (p2.id.clone(), p1.id.clone()),
            (p3.id.clone(), p2.id.clone()),
        ];

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone(), p3.clone()], deps);

        let result = service
            .validate_selection(&session_id, &[p1.id.clone(), p2.id.clone(), p3.id.clone()])
            .await
            .unwrap();

        assert!(result.is_valid);
        assert!(result.cycles.is_empty());
    }

    #[tokio::test]
    async fn test_validate_selection_with_cycle() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "A");
        let p2 = create_test_proposal(&session_id, "B");
        let p3 = create_test_proposal(&session_id, "C");

        // Circular: p1 -> p2 -> p3 -> p1
        let deps = vec![
            (p1.id.clone(), p2.id.clone()),
            (p2.id.clone(), p3.id.clone()),
            (p3.id.clone(), p1.id.clone()),
        ];

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone(), p3.clone()], deps);

        let result = service
            .validate_selection(&session_id, &[p1.id.clone(), p2.id.clone(), p3.id.clone()])
            .await
            .unwrap();

        assert!(!result.is_valid);
        assert!(!result.cycles.is_empty());
    }

    #[tokio::test]
    async fn test_validate_selection_missing_dependency_warning() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "Setup");
        let p2 = create_test_proposal(&session_id, "Feature");

        // p2 depends on p1, but we only select p2
        let deps = vec![(p2.id.clone(), p1.id.clone())];

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone()], deps);

        let result = service
            .validate_selection(&session_id, &[p2.id.clone()])
            .await
            .unwrap();

        assert!(result.is_valid); // Still valid, just a warning
        assert!(result.warnings.iter().any(|w| w.contains("not selected")));
    }

    // ========================================================================
    // TARGET COLUMN TESTS
    // ========================================================================

    #[test]
    fn test_target_column_to_status_draft() {
        assert_eq!(TargetColumn::Draft.to_status(), InternalStatus::Backlog);
    }

    #[test]
    fn test_target_column_to_status_backlog() {
        assert_eq!(TargetColumn::Backlog.to_status(), InternalStatus::Backlog);
    }

    #[test]
    fn test_target_column_to_status_todo() {
        assert_eq!(TargetColumn::Todo.to_status(), InternalStatus::Ready);
    }

    // ========================================================================
    // APPLY PROPOSALS TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_apply_proposals_creates_tasks() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone()], vec![]);

        let result = service
            .apply_proposals(
                &session_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone(), p2.id.clone()],
                    target_column: TargetColumn::Backlog,
                    preserve_dependencies: false,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.created_tasks.len(), 2);
        assert!(result.created_tasks.iter().any(|t| t.title == "Task 1"));
        assert!(result.created_tasks.iter().any(|t| t.title == "Task 2"));
    }

    #[tokio::test]
    async fn test_apply_proposals_sets_correct_status() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "Task 1");

        let service = create_service(session.clone(), vec![p1.clone()], vec![]);

        let result = service
            .apply_proposals(
                &session_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone()],
                    target_column: TargetColumn::Todo,
                    preserve_dependencies: false,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.created_tasks.len(), 1);
        assert_eq!(result.created_tasks[0].internal_status, InternalStatus::Ready);
    }

    #[tokio::test]
    async fn test_apply_proposals_preserves_dependencies() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "Setup");
        let p2 = create_test_proposal(&session_id, "Feature");

        // p2 depends on p1
        let deps = vec![(p2.id.clone(), p1.id.clone())];

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone()], deps);

        let result = service
            .apply_proposals(
                &session_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone(), p2.id.clone()],
                    target_column: TargetColumn::Backlog,
                    preserve_dependencies: true,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.created_tasks.len(), 2);
        assert_eq!(result.dependencies_created, 1);
    }

    #[tokio::test]
    async fn test_apply_proposals_no_dependencies_when_not_preserved() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "Setup");
        let p2 = create_test_proposal(&session_id, "Feature");

        let deps = vec![(p2.id.clone(), p1.id.clone())];

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone()], deps);

        let result = service
            .apply_proposals(
                &session_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone(), p2.id.clone()],
                    target_column: TargetColumn::Backlog,
                    preserve_dependencies: false,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.created_tasks.len(), 2);
        assert_eq!(result.dependencies_created, 0);
    }

    #[tokio::test]
    async fn test_apply_proposals_fails_for_archived_session() {
        let project_id = ProjectId::new();
        let mut session = IdeationSession::new(project_id.clone());
        session.archive();
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "Task 1");

        let service = create_service(session.clone(), vec![p1.clone()], vec![]);

        let result = service
            .apply_proposals(
                &session_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone()],
                    target_column: TargetColumn::Backlog,
                    preserve_dependencies: false,
                },
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_apply_proposals_fails_for_nonexistent_session() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let nonexistent_id = IdeationSessionId::new();

        let p1 = create_test_proposal(&session.id, "Task 1");

        let service = create_service(session.clone(), vec![p1.clone()], vec![]);

        let result = service
            .apply_proposals(
                &nonexistent_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone()],
                    target_column: TargetColumn::Backlog,
                    preserve_dependencies: false,
                },
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_apply_proposals_fails_with_circular_deps() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "A");
        let p2 = create_test_proposal(&session_id, "B");

        // Circular: p1 -> p2 -> p1
        let deps = vec![
            (p1.id.clone(), p2.id.clone()),
            (p2.id.clone(), p1.id.clone()),
        ];

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone()], deps);

        let result = service
            .apply_proposals(
                &session_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone(), p2.id.clone()],
                    target_column: TargetColumn::Backlog,
                    preserve_dependencies: true,
                },
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_apply_proposals_copies_fields_correctly() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let mut p1 = create_test_proposal(&session_id, "My Task");
        p1.description = Some("This is a description".to_string());
        p1.priority_score = 75;
        p1.category = TaskCategory::Fix;

        let service = create_service(session.clone(), vec![p1.clone()], vec![]);

        let result = service
            .apply_proposals(
                &session_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone()],
                    target_column: TargetColumn::Backlog,
                    preserve_dependencies: false,
                },
            )
            .await
            .unwrap();

        let task = &result.created_tasks[0];
        assert_eq!(task.title, "My Task");
        assert_eq!(task.description, Some("This is a description".to_string()));
        assert_eq!(task.priority, 75);
        assert_eq!(task.category, "fix");
    }

    // ========================================================================
    // APPLY SELECTED TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_apply_selected_proposals() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let mut p1 = create_test_proposal(&session_id, "Task 1");
        p1.selected = true;
        let mut p2 = create_test_proposal(&session_id, "Task 2");
        p2.selected = false; // Not selected
        let mut p3 = create_test_proposal(&session_id, "Task 3");
        p3.selected = true;

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone(), p3.clone()], vec![]);

        let result = service
            .apply_selected_proposals(&session_id, TargetColumn::Backlog, false)
            .await
            .unwrap();

        assert_eq!(result.created_tasks.len(), 2);
        assert!(result.created_tasks.iter().any(|t| t.title == "Task 1"));
        assert!(result.created_tasks.iter().any(|t| t.title == "Task 3"));
    }

    // ========================================================================
    // SESSION CONVERSION TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_apply_all_converts_session() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone()], vec![]);

        let result = service
            .apply_proposals(
                &session_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone(), p2.id.clone()],
                    target_column: TargetColumn::Backlog,
                    preserve_dependencies: false,
                },
            )
            .await
            .unwrap();

        assert!(result.session_converted);
    }

    #[tokio::test]
    async fn test_apply_partial_does_not_convert_session() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();

        let p1 = create_test_proposal(&session_id, "Task 1");
        let p2 = create_test_proposal(&session_id, "Task 2");

        let service = create_service(session.clone(), vec![p1.clone(), p2.clone()], vec![]);

        // Only apply p1, not p2
        let result = service
            .apply_proposals(
                &session_id,
                ApplyProposalsOptions {
                    proposal_ids: vec![p1.id.clone()],
                    target_column: TargetColumn::Backlog,
                    preserve_dependencies: false,
                },
            )
            .await
            .unwrap();

        assert!(!result.session_converted);
    }
}
