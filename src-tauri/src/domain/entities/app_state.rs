use crate::domain::entities::ProjectId;

#[derive(Debug, Clone, Default)]
pub struct AppSettings {
    pub active_project_id: Option<ProjectId>,
}
