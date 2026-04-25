use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::types::Type;
use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::domain::entities::{
    ChatConversationId, ChatMessageId, DesignApprovalStatus, DesignFeedbackStatus, DesignRun,
    DesignRunId, DesignSchemaVersion, DesignSchemaVersionId, DesignSourceRef, DesignStorageRootRef,
    DesignStyleguideFeedback, DesignStyleguideFeedbackId, DesignStyleguideItem,
    DesignStyleguideItemId, DesignSystem, DesignSystemId, DesignSystemSource, DesignSystemSourceId,
    ProjectId,
};
use crate::domain::repositories::{
    DesignRunRepository, DesignSchemaRepository, DesignStyleguideFeedbackRepository,
    DesignStyleguideRepository, DesignSystemRepository, DesignSystemSourceRepository,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::sqlite::DbConnection;

fn parse_datetime(value: String) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|datetime| datetime.with_timezone(&Utc))
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error)))
}

fn parse_optional_datetime(value: Option<String>) -> rusqlite::Result<Option<DateTime<Utc>>> {
    value.map(parse_datetime).transpose()
}

fn json_text<T: Serialize>(value: &T) -> AppResult<String> {
    serde_json::to_string(value)
        .map_err(|error| AppError::Database(format!("JSON serialization error: {error}")))
}

fn parse_json<T: DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    serde_json::from_str(&value)
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error)))
}

fn enum_text<T: Serialize>(value: &T) -> AppResult<String> {
    let value = serde_json::to_value(value)
        .map_err(|error| AppError::Database(format!("JSON serialization error: {error}")))?;
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| AppError::Database("Enum did not serialize to a string".to_string()))
}

fn parse_enum<T: DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|error| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error)))
}

fn row_to_design_system(row: &rusqlite::Row) -> rusqlite::Result<DesignSystem> {
    Ok(DesignSystem {
        id: DesignSystemId::from_string(row.get::<_, String>("id")?),
        primary_project_id: ProjectId::from_string(row.get::<_, String>("primary_project_id")?),
        name: row.get("name")?,
        description: row.get("description")?,
        status: parse_enum(row.get::<_, String>("status")?)?,
        current_schema_version_id: row
            .get::<_, Option<String>>("current_schema_version_id")?
            .map(DesignSchemaVersionId::from_string),
        storage_root_ref: DesignStorageRootRef::from_hash_component(
            row.get::<_, String>("storage_root_ref")?,
        ),
        created_at: parse_datetime(row.get("created_at")?)?,
        updated_at: parse_datetime(row.get("updated_at")?)?,
        archived_at: parse_optional_datetime(row.get("archived_at")?)?,
    })
}

fn row_to_design_source(row: &rusqlite::Row) -> rusqlite::Result<DesignSystemSource> {
    Ok(DesignSystemSource {
        id: DesignSystemSourceId::from_string(row.get::<_, String>("id")?),
        design_system_id: DesignSystemId::from_string(row.get::<_, String>("design_system_id")?),
        project_id: ProjectId::from_string(row.get::<_, String>("project_id")?),
        role: parse_enum(row.get::<_, String>("role")?)?,
        selected_paths: parse_json(row.get::<_, String>("selected_paths_json")?)?,
        source_kind: parse_enum(row.get::<_, String>("source_kind")?)?,
        git_commit: row.get("git_commit")?,
        source_hashes: parse_json::<BTreeMap<String, String>>(
            row.get::<_, String>("source_hashes_json")?,
        )?,
        last_analyzed_at: parse_optional_datetime(row.get("last_analyzed_at")?)?,
    })
}

fn row_to_schema_version(row: &rusqlite::Row) -> rusqlite::Result<DesignSchemaVersion> {
    Ok(DesignSchemaVersion {
        id: DesignSchemaVersionId::from_string(row.get::<_, String>("id")?),
        design_system_id: DesignSystemId::from_string(row.get::<_, String>("design_system_id")?),
        version: row.get("version")?,
        schema_artifact_id: row.get("schema_artifact_id")?,
        manifest_artifact_id: row.get("manifest_artifact_id")?,
        styleguide_artifact_id: row.get("styleguide_artifact_id")?,
        status: parse_enum(row.get::<_, String>("status")?)?,
        created_by_run_id: row
            .get::<_, Option<String>>("created_by_run_id")?
            .map(DesignRunId::from_string),
        created_at: parse_datetime(row.get("created_at")?)?,
    })
}

fn row_to_styleguide_item(row: &rusqlite::Row) -> rusqlite::Result<DesignStyleguideItem> {
    Ok(DesignStyleguideItem {
        id: DesignStyleguideItemId::from_string(row.get::<_, String>("id")?),
        design_system_id: DesignSystemId::from_string(row.get::<_, String>("design_system_id")?),
        schema_version_id: DesignSchemaVersionId::from_string(
            row.get::<_, String>("schema_version_id")?,
        ),
        item_id: row.get("item_id")?,
        group: parse_enum(row.get::<_, String>("group_name")?)?,
        label: row.get("label")?,
        summary: row.get("summary")?,
        preview_artifact_id: row.get("preview_artifact_id")?,
        source_refs: parse_json::<Vec<DesignSourceRef>>(row.get::<_, String>("source_refs_json")?)?,
        confidence: parse_enum(row.get::<_, String>("confidence")?)?,
        approval_status: parse_enum(row.get::<_, String>("approval_status")?)?,
        feedback_status: parse_enum(row.get::<_, String>("feedback_status")?)?,
        updated_at: parse_datetime(row.get("updated_at")?)?,
    })
}

fn row_to_feedback(row: &rusqlite::Row) -> rusqlite::Result<DesignStyleguideFeedback> {
    Ok(DesignStyleguideFeedback {
        id: DesignStyleguideFeedbackId::from_string(row.get::<_, String>("id")?),
        design_system_id: DesignSystemId::from_string(row.get::<_, String>("design_system_id")?),
        schema_version_id: DesignSchemaVersionId::from_string(
            row.get::<_, String>("schema_version_id")?,
        ),
        item_id: row.get("item_id")?,
        conversation_id: ChatConversationId::from_string(row.get::<_, String>("conversation_id")?),
        message_id: row
            .get::<_, Option<String>>("message_id")?
            .map(ChatMessageId::from_string),
        preview_artifact_id: row.get("preview_artifact_id")?,
        source_refs: parse_json::<Vec<DesignSourceRef>>(row.get::<_, String>("source_refs_json")?)?,
        feedback: row.get("feedback")?,
        status: parse_enum(row.get::<_, String>("status")?)?,
        created_at: parse_datetime(row.get("created_at")?)?,
        resolved_at: parse_optional_datetime(row.get("resolved_at")?)?,
    })
}

fn row_to_run(row: &rusqlite::Row) -> rusqlite::Result<DesignRun> {
    Ok(DesignRun {
        id: DesignRunId::from_string(row.get::<_, String>("id")?),
        design_system_id: DesignSystemId::from_string(row.get::<_, String>("design_system_id")?),
        conversation_id: row
            .get::<_, Option<String>>("conversation_id")?
            .map(ChatConversationId::from_string),
        kind: parse_enum(row.get::<_, String>("kind")?)?,
        status: parse_enum(row.get::<_, String>("status")?)?,
        input_summary: row.get("input_summary")?,
        output_artifact_ids: parse_json(row.get::<_, String>("output_artifact_ids_json")?)?,
        started_at: parse_optional_datetime(row.get("started_at")?)?,
        completed_at: parse_optional_datetime(row.get("completed_at")?)?,
        error: row.get("error")?,
    })
}

#[derive(Clone)]
pub struct SqliteDesignSystemRepository {
    db: DbConnection,
}

impl SqliteDesignSystemRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl DesignSystemRepository for SqliteDesignSystemRepository {
    async fn create(&self, system: DesignSystem) -> AppResult<DesignSystem> {
        let id = system.id.as_str().to_string();
        let primary_project_id = system.primary_project_id.as_str().to_string();
        let name = system.name.clone();
        let description = system.description.clone();
        let status = enum_text(&system.status)?;
        let current_schema_version_id = system
            .current_schema_version_id
            .as_ref()
            .map(|id| id.as_str().to_string());
        let storage_root_ref = system.storage_root_ref.as_str().to_string();
        let created_at = system.created_at.to_rfc3339();
        let updated_at = system.updated_at.to_rfc3339();
        let archived_at = system.archived_at.map(|value| value.to_rfc3339());

        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO design_systems (
                        id, primary_project_id, name, description, status,
                        current_schema_version_id, storage_root_ref, created_at, updated_at,
                        archived_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        id,
                        primary_project_id,
                        name,
                        description,
                        status,
                        current_schema_version_id,
                        storage_root_ref,
                        created_at,
                        updated_at,
                        archived_at,
                    ],
                )?;
                Ok(())
            })
            .await?;

        Ok(system)
    }

    async fn get_by_id(&self, id: &DesignSystemId) -> AppResult<Option<DesignSystem>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, primary_project_id, name, description, status,
                            current_schema_version_id, storage_root_ref, created_at, updated_at,
                            archived_at
                     FROM design_systems WHERE id = ?1",
                    [id],
                    row_to_design_system,
                )
            })
            .await
    }

    async fn list_by_project(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
    ) -> AppResult<Vec<DesignSystem>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let sql = if include_archived {
                    "SELECT id, primary_project_id, name, description, status,
                            current_schema_version_id, storage_root_ref, created_at, updated_at,
                            archived_at
                     FROM design_systems
                     WHERE primary_project_id = ?1
                     ORDER BY updated_at DESC"
                } else {
                    "SELECT id, primary_project_id, name, description, status,
                            current_schema_version_id, storage_root_ref, created_at, updated_at,
                            archived_at
                     FROM design_systems
                     WHERE primary_project_id = ?1 AND archived_at IS NULL
                     ORDER BY updated_at DESC"
                };
                let mut stmt = conn.prepare(sql)?;
                let rows = stmt.query_map([project_id], row_to_design_system)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(AppError::from)
            })
            .await
    }

    async fn update(&self, system: &DesignSystem) -> AppResult<()> {
        let id = system.id.as_str().to_string();
        let name = system.name.clone();
        let description = system.description.clone();
        let status = enum_text(&system.status)?;
        let current_schema_version_id = system
            .current_schema_version_id
            .as_ref()
            .map(|id| id.as_str().to_string());
        let updated_at = system.updated_at.to_rfc3339();
        let archived_at = system.archived_at.map(|value| value.to_rfc3339());

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE design_systems
                     SET name = ?2, description = ?3, status = ?4,
                         current_schema_version_id = ?5, updated_at = ?6, archived_at = ?7
                     WHERE id = ?1",
                    rusqlite::params![
                        id,
                        name,
                        description,
                        status,
                        current_schema_version_id,
                        updated_at,
                        archived_at,
                    ],
                )?;
                Ok(())
            })
            .await
    }

    async fn archive(&self, id: &DesignSystemId) -> AppResult<()> {
        let id = id.as_str().to_string();
        let archived_at = Utc::now().to_rfc3339();
        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE design_systems
                     SET archived_at = ?2, updated_at = ?2
                     WHERE id = ?1",
                    rusqlite::params![id, archived_at],
                )?;
                Ok(())
            })
            .await
    }
}

#[derive(Clone)]
pub struct SqliteDesignSystemSourceRepository {
    db: DbConnection,
}

impl SqliteDesignSystemSourceRepository {
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl DesignSystemSourceRepository for SqliteDesignSystemSourceRepository {
    async fn replace_for_design_system(
        &self,
        design_system_id: &DesignSystemId,
        sources: Vec<DesignSystemSource>,
    ) -> AppResult<()> {
        let design_system_id = design_system_id.as_str().to_string();
        let rows = sources
            .into_iter()
            .map(|source| {
                Ok((
                    source.id.as_str().to_string(),
                    source.design_system_id.as_str().to_string(),
                    source.project_id.as_str().to_string(),
                    enum_text(&source.role)?,
                    json_text(&source.selected_paths)?,
                    enum_text(&source.source_kind)?,
                    source.git_commit,
                    json_text(&source.source_hashes)?,
                    source.last_analyzed_at.map(|value| value.to_rfc3339()),
                ))
            })
            .collect::<AppResult<Vec<_>>>()?;

        self.db
            .run_transaction(move |conn| {
                conn.execute(
                    "DELETE FROM design_system_sources WHERE design_system_id = ?1",
                    [&design_system_id],
                )?;
                for row in rows {
                    conn.execute(
                        "INSERT INTO design_system_sources (
                            id, design_system_id, project_id, role, selected_paths_json,
                            source_kind, git_commit, source_hashes_json, last_analyzed_at
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                        rusqlite::params![
                            row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8,
                        ],
                    )?;
                }
                Ok(())
            })
            .await
    }

    async fn list_by_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignSystemSource>> {
        let design_system_id = design_system_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, design_system_id, project_id, role, selected_paths_json,
                            source_kind, git_commit, source_hashes_json, last_analyzed_at
                     FROM design_system_sources
                     WHERE design_system_id = ?1
                     ORDER BY role ASC, id ASC",
                )?;
                let rows = stmt.query_map([design_system_id], row_to_design_source)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(AppError::from)
            })
            .await
    }
}

#[derive(Clone)]
pub struct SqliteDesignSchemaRepository {
    db: DbConnection,
}

impl SqliteDesignSchemaRepository {
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl DesignSchemaRepository for SqliteDesignSchemaRepository {
    async fn create_version(&self, version: DesignSchemaVersion) -> AppResult<DesignSchemaVersion> {
        let id = version.id.as_str().to_string();
        let design_system_id = version.design_system_id.as_str().to_string();
        let version_label = version.version.clone();
        let schema_artifact_id = version.schema_artifact_id.clone();
        let manifest_artifact_id = version.manifest_artifact_id.clone();
        let styleguide_artifact_id = version.styleguide_artifact_id.clone();
        let status = enum_text(&version.status)?;
        let created_by_run_id = version
            .created_by_run_id
            .as_ref()
            .map(|id| id.as_str().to_string());
        let created_at = version.created_at.to_rfc3339();

        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO design_schema_versions (
                        id, design_system_id, version, schema_artifact_id, manifest_artifact_id,
                        styleguide_artifact_id, status, created_by_run_id, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        id,
                        design_system_id,
                        version_label,
                        schema_artifact_id,
                        manifest_artifact_id,
                        styleguide_artifact_id,
                        status,
                        created_by_run_id,
                        created_at,
                    ],
                )?;
                Ok(())
            })
            .await?;

        Ok(version)
    }

    async fn get_version(
        &self,
        id: &DesignSchemaVersionId,
    ) -> AppResult<Option<DesignSchemaVersion>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, design_system_id, version, schema_artifact_id,
                            manifest_artifact_id, styleguide_artifact_id, status,
                            created_by_run_id, created_at
                     FROM design_schema_versions WHERE id = ?1",
                    [id],
                    row_to_schema_version,
                )
            })
            .await
    }

    async fn get_current_for_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Option<DesignSchemaVersion>> {
        let design_system_id = design_system_id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT v.id, v.design_system_id, v.version, v.schema_artifact_id,
                            v.manifest_artifact_id, v.styleguide_artifact_id, v.status,
                            v.created_by_run_id, v.created_at
                     FROM design_systems s
                     JOIN design_schema_versions v ON v.id = s.current_schema_version_id
                     WHERE s.id = ?1",
                    [design_system_id],
                    row_to_schema_version,
                )
            })
            .await
    }

    async fn list_versions(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignSchemaVersion>> {
        let design_system_id = design_system_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, design_system_id, version, schema_artifact_id,
                            manifest_artifact_id, styleguide_artifact_id, status,
                            created_by_run_id, created_at
                     FROM design_schema_versions
                     WHERE design_system_id = ?1
                     ORDER BY created_at DESC",
                )?;
                let rows = stmt.query_map([design_system_id], row_to_schema_version)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(AppError::from)
            })
            .await
    }
}

#[derive(Clone)]
pub struct SqliteDesignStyleguideRepository {
    db: DbConnection,
}

impl SqliteDesignStyleguideRepository {
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl DesignStyleguideRepository for SqliteDesignStyleguideRepository {
    async fn replace_items_for_schema_version(
        &self,
        schema_version_id: &DesignSchemaVersionId,
        items: Vec<DesignStyleguideItem>,
    ) -> AppResult<()> {
        let schema_version_id = schema_version_id.as_str().to_string();
        let rows = items
            .into_iter()
            .map(|item| {
                Ok((
                    item.id.as_str().to_string(),
                    item.design_system_id.as_str().to_string(),
                    item.schema_version_id.as_str().to_string(),
                    item.item_id,
                    enum_text(&item.group)?,
                    item.label,
                    item.summary,
                    item.preview_artifact_id,
                    json_text(&item.source_refs)?,
                    enum_text(&item.confidence)?,
                    enum_text(&item.approval_status)?,
                    enum_text(&item.feedback_status)?,
                    item.updated_at.to_rfc3339(),
                ))
            })
            .collect::<AppResult<Vec<_>>>()?;

        self.db
            .run_transaction(move |conn| {
                conn.execute(
                    "DELETE FROM design_styleguide_items WHERE schema_version_id = ?1",
                    [&schema_version_id],
                )?;
                for row in rows {
                    conn.execute(
                        "INSERT INTO design_styleguide_items (
                            id, design_system_id, schema_version_id, item_id, group_name,
                            label, summary, preview_artifact_id, source_refs_json, confidence,
                            approval_status, feedback_status, updated_at
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                        rusqlite::params![
                            row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8, row.9,
                            row.10, row.11, row.12,
                        ],
                    )?;
                }
                Ok(())
            })
            .await
    }

    async fn list_items(
        &self,
        design_system_id: &DesignSystemId,
        schema_version_id: Option<&DesignSchemaVersionId>,
    ) -> AppResult<Vec<DesignStyleguideItem>> {
        let design_system_id = design_system_id.as_str().to_string();
        let schema_version_id = schema_version_id.map(|id| id.as_str().to_string());
        self.db
            .run(move |conn| {
                let mut items = Vec::new();
                if let Some(schema_version_id) = schema_version_id {
                    let mut stmt = conn.prepare(
                        "SELECT id, design_system_id, schema_version_id, item_id, group_name,
                                label, summary, preview_artifact_id, source_refs_json, confidence,
                                approval_status, feedback_status, updated_at
                         FROM design_styleguide_items
                         WHERE design_system_id = ?1 AND schema_version_id = ?2
                         ORDER BY group_name ASC, item_id ASC",
                    )?;
                    let rows = stmt.query_map(
                        rusqlite::params![design_system_id, schema_version_id],
                        row_to_styleguide_item,
                    )?;
                    for row in rows {
                        items.push(row?);
                    }
                } else {
                    let mut stmt = conn.prepare(
                        "SELECT id, design_system_id, schema_version_id, item_id, group_name,
                                label, summary, preview_artifact_id, source_refs_json, confidence,
                                approval_status, feedback_status, updated_at
                         FROM design_styleguide_items
                         WHERE design_system_id = ?1
                         ORDER BY group_name ASC, item_id ASC",
                    )?;
                    let rows = stmt.query_map([design_system_id], row_to_styleguide_item)?;
                    for row in rows {
                        items.push(row?);
                    }
                }
                Ok(items)
            })
            .await
    }

    async fn get_item(
        &self,
        design_system_id: &DesignSystemId,
        item_id: &str,
    ) -> AppResult<Option<DesignStyleguideItem>> {
        let design_system_id = design_system_id.as_str().to_string();
        let item_id = item_id.to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, design_system_id, schema_version_id, item_id, group_name,
                            label, summary, preview_artifact_id, source_refs_json, confidence,
                            approval_status, feedback_status, updated_at
                     FROM design_styleguide_items
                     WHERE design_system_id = ?1 AND item_id = ?2
                     ORDER BY updated_at DESC
                     LIMIT 1",
                    rusqlite::params![design_system_id, item_id],
                    row_to_styleguide_item,
                )
            })
            .await
    }

    async fn update_item(&self, item: &DesignStyleguideItem) -> AppResult<()> {
        let id = item.id.as_str().to_string();
        let label = item.label.clone();
        let summary = item.summary.clone();
        let preview_artifact_id = item.preview_artifact_id.clone();
        let source_refs_json = json_text(&item.source_refs)?;
        let confidence = enum_text(&item.confidence)?;
        let approval_status = enum_text(&item.approval_status)?;
        let feedback_status = enum_text(&item.feedback_status)?;
        let updated_at = item.updated_at.to_rfc3339();

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE design_styleguide_items
                     SET label = ?2, summary = ?3, preview_artifact_id = ?4,
                         source_refs_json = ?5, confidence = ?6, approval_status = ?7,
                         feedback_status = ?8, updated_at = ?9
                     WHERE id = ?1",
                    rusqlite::params![
                        id,
                        label,
                        summary,
                        preview_artifact_id,
                        source_refs_json,
                        confidence,
                        approval_status,
                        feedback_status,
                        updated_at,
                    ],
                )?;
                Ok(())
            })
            .await
    }

    async fn approve_item(&self, id: &DesignStyleguideItemId) -> AppResult<()> {
        let id = id.as_str().to_string();
        let approval_status = enum_text(&DesignApprovalStatus::Approved)?;
        let feedback_status = enum_text(&DesignFeedbackStatus::Resolved)?;
        let updated_at = Utc::now().to_rfc3339();
        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE design_styleguide_items
                     SET approval_status = ?2, feedback_status = ?3, updated_at = ?4
                     WHERE id = ?1",
                    rusqlite::params![id, approval_status, feedback_status, updated_at],
                )?;
                Ok(())
            })
            .await
    }
}

#[derive(Clone)]
pub struct SqliteDesignStyleguideFeedbackRepository {
    db: DbConnection,
}

impl SqliteDesignStyleguideFeedbackRepository {
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl DesignStyleguideFeedbackRepository for SqliteDesignStyleguideFeedbackRepository {
    async fn create(
        &self,
        feedback: DesignStyleguideFeedback,
    ) -> AppResult<DesignStyleguideFeedback> {
        let id = feedback.id.as_str().to_string();
        let design_system_id = feedback.design_system_id.as_str().to_string();
        let schema_version_id = feedback.schema_version_id.as_str().to_string();
        let item_id = feedback.item_id.clone();
        let conversation_id = feedback.conversation_id.as_str().to_string();
        let message_id = feedback
            .message_id
            .as_ref()
            .map(|id| id.as_str().to_string());
        let preview_artifact_id = feedback.preview_artifact_id.clone();
        let source_refs_json = json_text(&feedback.source_refs)?;
        let feedback_text = feedback.feedback.clone();
        let status = enum_text(&feedback.status)?;
        let created_at = feedback.created_at.to_rfc3339();
        let resolved_at = feedback.resolved_at.map(|value| value.to_rfc3339());

        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO design_styleguide_feedback (
                        id, design_system_id, schema_version_id, item_id, conversation_id,
                        message_id, preview_artifact_id, source_refs_json, feedback, status,
                        created_at, resolved_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    rusqlite::params![
                        id,
                        design_system_id,
                        schema_version_id,
                        item_id,
                        conversation_id,
                        message_id,
                        preview_artifact_id,
                        source_refs_json,
                        feedback_text,
                        status,
                        created_at,
                        resolved_at,
                    ],
                )?;
                Ok(())
            })
            .await?;

        Ok(feedback)
    }

    async fn get_by_id(
        &self,
        id: &DesignStyleguideFeedbackId,
    ) -> AppResult<Option<DesignStyleguideFeedback>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, design_system_id, schema_version_id, item_id, conversation_id,
                            message_id, preview_artifact_id, source_refs_json, feedback, status,
                            created_at, resolved_at
                     FROM design_styleguide_feedback WHERE id = ?1",
                    [id],
                    row_to_feedback,
                )
            })
            .await
    }

    async fn list_open_by_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignStyleguideFeedback>> {
        let design_system_id = design_system_id.as_str().to_string();
        let resolved = enum_text(&DesignFeedbackStatus::Resolved)?;
        let dismissed = enum_text(&DesignFeedbackStatus::Dismissed)?;
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, design_system_id, schema_version_id, item_id, conversation_id,
                            message_id, preview_artifact_id, source_refs_json, feedback, status,
                            created_at, resolved_at
                     FROM design_styleguide_feedback
                     WHERE design_system_id = ?1 AND status NOT IN (?2, ?3)
                     ORDER BY created_at DESC",
                )?;
                let rows = stmt.query_map(
                    rusqlite::params![design_system_id, resolved, dismissed],
                    row_to_feedback,
                )?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(AppError::from)
            })
            .await
    }

    async fn update(&self, feedback: &DesignStyleguideFeedback) -> AppResult<()> {
        let id = feedback.id.as_str().to_string();
        let message_id = feedback
            .message_id
            .as_ref()
            .map(|id| id.as_str().to_string());
        let preview_artifact_id = feedback.preview_artifact_id.clone();
        let source_refs_json = json_text(&feedback.source_refs)?;
        let feedback_text = feedback.feedback.clone();
        let status = enum_text(&feedback.status)?;
        let resolved_at = feedback.resolved_at.map(|value| value.to_rfc3339());

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE design_styleguide_feedback
                     SET message_id = ?2, preview_artifact_id = ?3, source_refs_json = ?4,
                         feedback = ?5, status = ?6, resolved_at = ?7
                     WHERE id = ?1",
                    rusqlite::params![
                        id,
                        message_id,
                        preview_artifact_id,
                        source_refs_json,
                        feedback_text,
                        status,
                        resolved_at,
                    ],
                )?;
                Ok(())
            })
            .await
    }
}

#[derive(Clone)]
pub struct SqliteDesignRunRepository {
    db: DbConnection,
}

impl SqliteDesignRunRepository {
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl DesignRunRepository for SqliteDesignRunRepository {
    async fn create(&self, run: DesignRun) -> AppResult<DesignRun> {
        let id = run.id.as_str().to_string();
        let design_system_id = run.design_system_id.as_str().to_string();
        let conversation_id = run
            .conversation_id
            .as_ref()
            .map(|id| id.as_str().to_string());
        let kind = enum_text(&run.kind)?;
        let status = enum_text(&run.status)?;
        let input_summary = run.input_summary.clone();
        let output_artifact_ids_json = json_text(&run.output_artifact_ids)?;
        let started_at = run.started_at.map(|value| value.to_rfc3339());
        let completed_at = run.completed_at.map(|value| value.to_rfc3339());
        let error = run.error.clone();

        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO design_runs (
                        id, design_system_id, conversation_id, kind, status, input_summary,
                        output_artifact_ids_json, started_at, completed_at, error
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        id,
                        design_system_id,
                        conversation_id,
                        kind,
                        status,
                        input_summary,
                        output_artifact_ids_json,
                        started_at,
                        completed_at,
                        error,
                    ],
                )?;
                Ok(())
            })
            .await?;

        Ok(run)
    }

    async fn get_by_id(&self, id: &DesignRunId) -> AppResult<Option<DesignRun>> {
        let id = id.as_str().to_string();
        self.db
            .query_optional(move |conn| {
                conn.query_row(
                    "SELECT id, design_system_id, conversation_id, kind, status, input_summary,
                            output_artifact_ids_json, started_at, completed_at, error
                     FROM design_runs WHERE id = ?1",
                    [id],
                    row_to_run,
                )
            })
            .await
    }

    async fn list_by_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignRun>> {
        let design_system_id = design_system_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, design_system_id, conversation_id, kind, status, input_summary,
                            output_artifact_ids_json, started_at, completed_at, error
                     FROM design_runs
                     WHERE design_system_id = ?1
                     ORDER BY COALESCE(started_at, completed_at) DESC, id ASC",
                )?;
                let rows = stmt.query_map([design_system_id], row_to_run)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(AppError::from)
            })
            .await
    }

    async fn update(&self, run: &DesignRun) -> AppResult<()> {
        let id = run.id.as_str().to_string();
        let conversation_id = run
            .conversation_id
            .as_ref()
            .map(|id| id.as_str().to_string());
        let status = enum_text(&run.status)?;
        let input_summary = run.input_summary.clone();
        let output_artifact_ids_json = json_text(&run.output_artifact_ids)?;
        let started_at = run.started_at.map(|value| value.to_rfc3339());
        let completed_at = run.completed_at.map(|value| value.to_rfc3339());
        let error = run.error.clone();

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE design_runs
                     SET conversation_id = ?2, status = ?3, input_summary = ?4,
                         output_artifact_ids_json = ?5, started_at = ?6, completed_at = ?7,
                         error = ?8
                     WHERE id = ?1",
                    rusqlite::params![
                        id,
                        conversation_id,
                        status,
                        input_summary,
                        output_artifact_ids_json,
                        started_at,
                        completed_at,
                        error,
                    ],
                )?;
                Ok(())
            })
            .await
    }
}
