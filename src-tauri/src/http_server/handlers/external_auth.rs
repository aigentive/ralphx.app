// ValidatedExternalKey Axum extractor for external MCP permission enforcement.
//
// Reads `X-RalphX-External-MCP` and `X-RalphX-Key-Id` headers, loads the API key
// from the DB by ID, and validates that the key has the required permission.
//
// Usage in handlers:
// ```rust,ignore
// pub async fn my_handler(
//     State(state): State<HttpServerState>,
//     validated_key: ValidatedExternalKey,
//     Json(req): Json<MyRequest>,
// ) -> Result<Json<MyResponse>, HttpError> { ... }
// ```

use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

use crate::domain::entities::{ApiKeyId, PERMISSION_CREATE_PROJECT};
use crate::http_server::types::{HttpError, HttpServerState};

/// Header that identifies a request as coming from the external MCP server.
/// Must be present and equal to "1" for `ValidatedExternalKey` to proceed.
pub const EXTERNAL_MCP_HEADER: &str = "x-ralphx-external-mcp";

/// Header carrying the opaque API key ID (UUID), injected by the external MCP server.
/// The extractor uses this to load the key from the DB and validate permissions.
pub const EXTERNAL_KEY_ID_HEADER: &str = "x-ralphx-key-id";

/// A validated external API key with CREATE_PROJECT permission.
///
/// This Axum extractor enforces that:
/// 1. The request carries `X-RalphX-External-MCP: 1`
/// 2. The request carries a valid `X-RalphX-Key-Id` header
/// 3. The referenced API key exists and is active
/// 4. The key has the `CREATE_PROJECT` permission bit set
///
/// Handler parameters of this type receive the key_id and permissions for
/// audit logging and fine-grained permission checks.
// Fields are intentionally public for use by downstream handlers (task: register_project_external)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ValidatedExternalKey {
    pub key_id: String,
    pub permissions: i32,
}

#[async_trait]
impl FromRequestParts<HttpServerState> for ValidatedExternalKey {
    type Rejection = HttpError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &HttpServerState,
    ) -> Result<Self, Self::Rejection> {
        // 1. Verify this is an external MCP request
        let is_external_mcp = parts
            .headers
            .get(EXTERNAL_MCP_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|v| v == "1")
            .unwrap_or(false);

        if !is_external_mcp {
            return Err(HttpError {
                status: StatusCode::UNAUTHORIZED,
                message: Some("Not an external MCP request".to_string()),
            });
        }

        // 2. Extract the key ID header
        let key_id_str = parts
            .headers
            .get(EXTERNAL_KEY_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| HttpError {
                status: StatusCode::UNAUTHORIZED,
                message: Some("Missing X-RalphX-Key-Id header".to_string()),
            })?;

        // 3. Load key from DB using newtype conversion
        let api_key_id = ApiKeyId::from_string(key_id_str.clone());
        let key = state
            .app_state
            .api_key_repo
            .get_by_id(&api_key_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to load API key for external auth: {}", e);
                HttpError {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: Some("Internal server error".to_string()),
                }
            })?
            .ok_or_else(|| HttpError {
                status: StatusCode::UNAUTHORIZED,
                message: Some("API key not found".to_string()),
            })?;

        // 4. Check key is active
        if !key.is_active() && !key.is_in_grace_period() {
            return Err(HttpError {
                status: StatusCode::UNAUTHORIZED,
                message: Some("API key has been revoked".to_string()),
            });
        }

        // 5. Validate CREATE_PROJECT permission
        if !key.has_permission(PERMISSION_CREATE_PROJECT) {
            tracing::warn!(
                key_id = %key_id_str,
                actual_permissions = key.permissions,
                required = PERMISSION_CREATE_PROJECT,
                "CREATE_PROJECT permission check failed"
            );
            return Err(HttpError {
                status: StatusCode::FORBIDDEN,
                message: Some("CREATE_PROJECT permission required".to_string()),
            });
        }

        Ok(ValidatedExternalKey {
            key_id: key_id_str,
            permissions: key.permissions,
        })
    }
}
