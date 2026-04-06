use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::Connection;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::entities::artifact::{
    Artifact, ArtifactBucketId, ArtifactType, TeamArtifactMetadata,
};
use crate::domain::entities::verification_critic_result::{CriticKind, VerificationCriticResult};
use crate::domain::repositories::verification_critic_result_repo::{
    SubmitCriticResultInput, SubmitCriticResultOutput, VerificationCriticResultRepo,
};
use crate::error::{AppError, AppResult};
use crate::infrastructure::sqlite::sqlite_artifact_repo::SqliteArtifactRepository;
use crate::infrastructure::sqlite::DbConnection;

pub struct SqliteVerificationCriticResultRepository {
    db: DbConnection,
}

impl SqliteVerificationCriticResultRepository {
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

/// Map a rusqlite UNIQUE constraint violation (extended_code 2067) to `AppError::Conflict`.
fn map_insert_err(err: rusqlite::Error) -> AppError {
    if let rusqlite::Error::SqliteFailure(ffi_err, _) = &err {
        if ffi_err.extended_code == 2067 {
            return AppError::Conflict(
                "Duplicate submission for (parent_session_id, generation, round, critic_kind)"
                    .to_string(),
            );
        }
    }
    AppError::Database(err.to_string())
}

fn resolve_artifact_type(artifact_type: Option<&str>) -> ArtifactType {
    match artifact_type.unwrap_or("TeamResearch") {
        "TeamAnalysis" => ArtifactType::TeamAnalysis,
        "TeamSummary" => ArtifactType::TeamSummary,
        _ => ArtifactType::TeamResearch,
    }
}

#[async_trait]
impl VerificationCriticResultRepo for SqliteVerificationCriticResultRepository {
    async fn submit(
        &self,
        input: SubmitCriticResultInput,
    ) -> AppResult<SubmitCriticResultOutput> {
        self.db
            .run_transaction(move |conn| {
                let artifact_type =
                    resolve_artifact_type(input.artifact_type.as_deref());

                let mut artifact = Artifact::new_inline(
                    &input.title,
                    artifact_type,
                    &input.content,
                    "team-lead",
                );
                artifact.bucket_id =
                    Some(ArtifactBucketId::from_string("team-findings".to_string()));
                artifact.metadata.team_metadata = Some(TeamArtifactMetadata {
                    team_name: "team".to_string(),
                    author_teammate: "team-lead".to_string(),
                    session_id: Some(input.parent_session_id.clone()),
                    team_phase: None,
                });

                let artifact_id = artifact.id.to_string();
                SqliteArtifactRepository::create_sync(conn, artifact)?;

                let result_id = Uuid::new_v4().to_string();
                let now = chrono::Utc::now().to_rfc3339();

                conn.execute(
                    "INSERT INTO verification_critic_results \
                     (id, parent_session_id, verification_session_id, \
                      verification_generation, round, critic_kind, \
                      artifact_id, status, created_at, updated_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        result_id,
                        input.parent_session_id,
                        input.verification_session_id,
                        input.verification_generation,
                        input.round,
                        input.critic_kind.as_str(),
                        artifact_id,
                        "complete",
                        now,
                        now,
                    ],
                )
                .map_err(map_insert_err)?;

                Ok(SubmitCriticResultOutput {
                    artifact_id,
                    result_id,
                })
            })
            .await
    }

    async fn get_round_results(
        &self,
        parent_session_id: &str,
        generation: i32,
        round: i32,
    ) -> AppResult<Vec<VerificationCriticResult>> {
        let parent_session_id = parent_session_id.to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, parent_session_id, verification_session_id, \
                            verification_generation, round, critic_kind, \
                            artifact_id, status, created_at, updated_at \
                     FROM verification_critic_results \
                     WHERE parent_session_id = ?1 \
                       AND verification_generation = ?2 \
                       AND round = ?3",
                )?;

                let rows = stmt
                    .query_map(rusqlite::params![parent_session_id, generation, round], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i32>(3)?,
                            row.get::<_, i32>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, String>(6)?,
                            row.get::<_, String>(7)?,
                            row.get::<_, String>(8)?,
                            row.get::<_, String>(9)?,
                        ))
                    })?
                    .map(|r| r.map_err(AppError::from))
                    .collect::<AppResult<Vec<_>>>()?;

                rows.into_iter()
                    .map(
                        |(
                            id,
                            parent_sid,
                            vsid,
                            vgen,
                            round_val,
                            critic_kind_str,
                            artifact_id,
                            status,
                            created_at,
                            updated_at,
                        )| {
                            let critic_kind = CriticKind::from_db_str(&critic_kind_str)
                                .ok_or_else(|| {
                                    AppError::Database(format!(
                                        "Unknown critic_kind in DB: {}",
                                        critic_kind_str
                                    ))
                                })?;
                            Ok(VerificationCriticResult {
                                id,
                                parent_session_id: parent_sid,
                                verification_session_id: vsid,
                                verification_generation: vgen,
                                round: round_val,
                                critic_kind,
                                artifact_id,
                                status,
                                created_at,
                                updated_at,
                            })
                        },
                    )
                    .collect()
            })
            .await
    }
}

#[cfg(test)]
#[path = "sqlite_verification_critic_result_repo_tests.rs"]
mod tests;
