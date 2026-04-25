use std::path::{Component, Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::entities::{DesignSchemaVersionId, DesignStorageRootRef, DesignSystemId};
use crate::error::{AppError, AppResult};

const DESIGN_ROOT_DIR: &str = "design-systems";
const DESIGN_REF_PREFIX: &str = "design-";
const DESIGN_REF_HEX_BYTES: usize = 12;
const DESIGN_REF_HEX_LEN: usize = DESIGN_REF_HEX_BYTES * 2;

/// RalphX-owned filesystem roots for generated design-system artifacts.
///
/// The active project checkout is only a source of evidence. Generated schemas,
/// previews, exports, and derived assets must be written below this app-data root.
#[derive(Debug, Clone)]
pub struct DesignStoragePaths {
    app_data_dir: PathBuf,
}

impl DesignStoragePaths {
    pub fn new(app_data_dir: impl AsRef<Path>) -> AppResult<Self> {
        let app_data_dir = app_data_dir.as_ref().canonicalize().map_err(|error| {
            AppError::Infrastructure(format!("Failed to canonicalize app data dir: {error}"))
        })?;
        Ok(Self { app_data_dir })
    }

    pub fn storage_ref_for_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> DesignStorageRootRef {
        DesignStorageRootRef::from_hash_component(hashed_component(design_system_id.as_str()))
    }

    pub fn ensure_design_system_root(
        &self,
        storage_ref: &DesignStorageRootRef,
    ) -> AppResult<PathBuf> {
        let component = validated_storage_component(storage_ref)?;
        let design_root = self.app_data_dir.join(DESIGN_ROOT_DIR);

        std::fs::create_dir_all(&design_root).map_err(|error| {
            AppError::Infrastructure(format!("Failed to create design storage root: {error}"))
        })?;
        let canonical_design_root = design_root.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design storage root: {error}"
            ))
        })?;
        ensure_under(
            &canonical_design_root,
            &self.app_data_dir,
            "design storage root",
        )?;

        let target = canonical_design_root.join(component);
        let parent = target.parent().ok_or_else(|| {
            AppError::Validation("Design storage target has no parent".to_string())
        })?;
        ensure_under(
            parent,
            &canonical_design_root,
            "design storage target parent",
        )?;

        std::fs::create_dir_all(&target).map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to create design system storage root: {error}"
            ))
        })?;
        let canonical_target = target.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design system storage root: {error}"
            ))
        })?;
        ensure_under(
            &canonical_target,
            &canonical_design_root,
            "design system storage root",
        )?;

        Ok(canonical_target)
    }

    pub fn child_path(
        &self,
        storage_root: &Path,
        relative_path: impl AsRef<Path>,
    ) -> AppResult<PathBuf> {
        let relative_path = validated_relative_path(relative_path.as_ref())?;
        let canonical_root = storage_root.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design storage root: {error}"
            ))
        })?;
        ensure_under(&canonical_root, &self.app_data_dir, "design storage root")?;

        let target = canonical_root.join(relative_path);
        let parent = target.parent().ok_or_else(|| {
            AppError::Validation("Design storage child path has no parent".to_string())
        })?;
        ensure_under(parent, &canonical_root, "design storage child parent")?;
        Ok(target)
    }

    pub fn write_json_file<T: Serialize>(
        &self,
        storage_root: &Path,
        relative_path: impl AsRef<Path>,
        value: &T,
    ) -> AppResult<PathBuf> {
        let target = self.child_path(storage_root, relative_path)?;
        let canonical_root = storage_root.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design storage root: {error}"
            ))
        })?;
        ensure_under(&canonical_root, &self.app_data_dir, "design storage root")?;

        let parent = target.parent().ok_or_else(|| {
            AppError::Validation("Design storage JSON path has no parent".to_string())
        })?;
        ensure_under(parent, &canonical_root, "design storage JSON parent")?;
        std::fs::create_dir_all(parent).map_err(|error| {
            AppError::Infrastructure(format!("Failed to create design JSON parent: {error}"))
        })?;
        let canonical_parent = parent.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design JSON parent: {error}"
            ))
        })?;
        ensure_under(
            &canonical_parent,
            &canonical_root,
            "design storage JSON parent",
        )?;

        let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
            AppError::Infrastructure(format!("Failed to serialize design JSON: {error}"))
        })?;
        std::fs::write(&target, bytes).map_err(|error| {
            AppError::Infrastructure(format!("Failed to write design JSON file: {error}"))
        })?;
        let canonical_target = target.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!("Failed to canonicalize design JSON file: {error}"))
        })?;
        ensure_under(
            &canonical_target,
            &canonical_root,
            "design storage JSON file",
        )?;
        Ok(canonical_target)
    }

    pub fn write_file(
        &self,
        storage_root: &Path,
        relative_path: impl AsRef<Path>,
        bytes: &[u8],
    ) -> AppResult<PathBuf> {
        let target = self.child_path(storage_root, relative_path)?;
        let canonical_root = storage_root.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design storage root: {error}"
            ))
        })?;
        ensure_under(&canonical_root, &self.app_data_dir, "design storage root")?;

        let parent = target.parent().ok_or_else(|| {
            AppError::Validation("Design storage file path has no parent".to_string())
        })?;
        ensure_under(parent, &canonical_root, "design storage file parent")?;
        std::fs::create_dir_all(parent).map_err(|error| {
            AppError::Infrastructure(format!("Failed to create design file parent: {error}"))
        })?;
        let canonical_parent = parent.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design file parent: {error}"
            ))
        })?;
        ensure_under(
            &canonical_parent,
            &canonical_root,
            "design storage file parent",
        )?;

        std::fs::write(&target, bytes).map_err(|error| {
            AppError::Infrastructure(format!("Failed to write design file: {error}"))
        })?;
        let canonical_target = target.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!("Failed to canonicalize design file: {error}"))
        })?;
        ensure_under(&canonical_target, &canonical_root, "design storage file")?;
        Ok(canonical_target)
    }

    pub fn read_json_file<T: DeserializeOwned>(
        &self,
        storage_root: &Path,
        file_path: impl AsRef<Path>,
    ) -> AppResult<T> {
        let file_path = file_path.as_ref();
        if !file_path.is_absolute() {
            return Err(AppError::Validation(
                "Design artifact file path must be absolute".to_string(),
            ));
        }

        let canonical_root = storage_root.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design storage root: {error}"
            ))
        })?;
        ensure_under(&canonical_root, &self.app_data_dir, "design storage root")?;

        let canonical_file = file_path.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design artifact file: {error}"
            ))
        })?;
        ensure_under(
            &canonical_file,
            &canonical_root,
            "design artifact JSON file",
        )?;

        let bytes = std::fs::read(&canonical_file).map_err(|error| {
            AppError::Infrastructure(format!("Failed to read design JSON file: {error}"))
        })?;
        serde_json::from_slice(&bytes).map_err(|error| {
            AppError::Infrastructure(format!("Failed to parse design JSON file: {error}"))
        })
    }

    pub fn read_json_file_under_design_storage_root<T: DeserializeOwned>(
        &self,
        file_path: impl AsRef<Path>,
    ) -> AppResult<T> {
        let file_path = file_path.as_ref();
        if !file_path.is_absolute() {
            return Err(AppError::Validation(
                "Design artifact file path must be absolute".to_string(),
            ));
        }

        let design_root = self.app_data_dir.join(DESIGN_ROOT_DIR);
        let canonical_design_root = design_root.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design storage root: {error}"
            ))
        })?;
        ensure_under(
            &canonical_design_root,
            &self.app_data_dir,
            "design storage root",
        )?;

        let canonical_file = file_path.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design artifact file: {error}"
            ))
        })?;
        ensure_under(
            &canonical_file,
            &canonical_design_root,
            "design artifact JSON file",
        )?;

        let bytes = std::fs::read(&canonical_file).map_err(|error| {
            AppError::Infrastructure(format!("Failed to read design JSON file: {error}"))
        })?;
        serde_json::from_slice(&bytes).map_err(|error| {
            AppError::Infrastructure(format!("Failed to parse design JSON file: {error}"))
        })
    }

    pub fn read_file_under_design_storage_root(
        &self,
        file_path: impl AsRef<Path>,
    ) -> AppResult<Vec<u8>> {
        let file_path = file_path.as_ref();
        if !file_path.is_absolute() {
            return Err(AppError::Validation(
                "Design artifact file path must be absolute".to_string(),
            ));
        }

        let design_root = self.app_data_dir.join(DESIGN_ROOT_DIR);
        let canonical_design_root = design_root.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design storage root: {error}"
            ))
        })?;
        ensure_under(
            &canonical_design_root,
            &self.app_data_dir,
            "design storage root",
        )?;

        let canonical_file = file_path.canonicalize().map_err(|error| {
            AppError::Infrastructure(format!(
                "Failed to canonicalize design artifact file: {error}"
            ))
        })?;
        ensure_under(
            &canonical_file,
            &canonical_design_root,
            "design artifact file",
        )?;

        std::fs::read(&canonical_file).map_err(|error| {
            AppError::Infrastructure(format!("Failed to read design file: {error}"))
        })
    }

    pub fn schema_version_component(&self, schema_version_id: &DesignSchemaVersionId) -> String {
        hashed_component(schema_version_id.as_str())
    }

    pub fn styleguide_item_component(&self, item_id: &str) -> String {
        hashed_component(item_id)
    }
}

fn hashed_component(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut encoded = String::with_capacity(DESIGN_REF_HEX_LEN);
    for byte in &digest[..DESIGN_REF_HEX_BYTES] {
        use std::fmt::Write as _;
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    format!("{DESIGN_REF_PREFIX}{encoded}")
}

fn validated_storage_component(storage_ref: &DesignStorageRootRef) -> AppResult<&str> {
    let component = storage_ref.as_str();
    let hex = component
        .strip_prefix(DESIGN_REF_PREFIX)
        .ok_or_else(|| AppError::Validation("Design storage ref has invalid prefix".to_string()))?;
    if hex.len() != DESIGN_REF_HEX_LEN || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::Validation(
            "Design storage ref has invalid digest".to_string(),
        ));
    }
    Ok(component)
}

fn validated_relative_path(path: &Path) -> AppResult<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(AppError::Validation(
            "Design storage child path cannot be empty".to_string(),
        ));
    }
    if path.is_absolute() {
        return Err(AppError::Validation(
            "Design storage child path must be relative".to_string(),
        ));
    }

    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => safe.push(value),
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(AppError::Validation(
                    "Design storage child path contains unsafe components".to_string(),
                ));
            }
        }
    }
    if safe.as_os_str().is_empty() {
        return Err(AppError::Validation(
            "Design storage child path cannot be empty".to_string(),
        ));
    }
    Ok(safe)
}

fn ensure_under(path: &Path, root: &Path, label: &str) -> AppResult<()> {
    if path.starts_with(root) {
        return Ok(());
    }
    Err(AppError::Validation(format!(
        "{label} escaped RalphX-owned design storage"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_storage_ref_hashes_raw_design_system_ids() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let storage_ref = paths.storage_ref_for_design_system(&DesignSystemId::from_string(
            "../unsafe/design".to_string(),
        ));

        assert!(storage_ref.as_str().starts_with(DESIGN_REF_PREFIX));
        assert!(!storage_ref.as_str().contains(".."));
        assert!(!storage_ref.as_str().contains('/'));
        assert!(!storage_ref.as_str().contains('\\'));
    }

    #[test]
    fn ensure_design_system_root_stays_under_app_data_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let storage_ref = paths
            .storage_ref_for_design_system(&DesignSystemId::from_string("design-1".to_string()));

        let root = paths
            .ensure_design_system_root(&storage_ref)
            .expect("design root");

        assert!(root.starts_with(temp.path().canonicalize().unwrap()));
        assert!(root.ends_with(storage_ref.as_str()));
    }

    #[test]
    fn ensure_design_system_root_rejects_forged_refs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = DesignStoragePaths::new(temp.path()).expect("storage paths");

        let result = paths
            .ensure_design_system_root(&DesignStorageRootRef::from_hash_component("../outside"));

        assert!(result.is_err());
    }

    #[test]
    fn child_path_rejects_absolute_and_parent_components() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let storage_ref = paths
            .storage_ref_for_design_system(&DesignSystemId::from_string("design-1".to_string()));
        let root = paths
            .ensure_design_system_root(&storage_ref)
            .expect("design root");

        assert!(paths.child_path(&root, "../outside.json").is_err());
        assert!(paths.child_path(&root, "/tmp/outside.json").is_err());
    }

    #[test]
    fn child_path_accepts_nested_relative_paths() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let storage_ref = paths
            .storage_ref_for_design_system(&DesignSystemId::from_string("design-1".to_string()));
        let root = paths
            .ensure_design_system_root(&storage_ref)
            .expect("design root");

        let child = paths
            .child_path(&root, "schemas/current.json")
            .expect("child path");

        assert!(child.starts_with(&root));
        assert!(child.ends_with("schemas/current.json"));
    }

    #[test]
    fn write_json_file_persists_inside_design_storage() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let storage_ref = paths
            .storage_ref_for_design_system(&DesignSystemId::from_string("design-1".to_string()));
        let root = paths
            .ensure_design_system_root(&storage_ref)
            .expect("design root");

        let target = paths
            .write_json_file(
                &root,
                "schema/current.json",
                &serde_json::json!({ "ok": true }),
            )
            .expect("write json");

        assert!(target.starts_with(&root));
        assert_eq!(
            std::fs::read_to_string(target).expect("json file"),
            "{\n  \"ok\": true\n}"
        );
    }

    #[test]
    fn read_json_file_rejects_paths_outside_design_storage() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        let paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let storage_ref = paths
            .storage_ref_for_design_system(&DesignSystemId::from_string("design-1".to_string()));
        let root = paths
            .ensure_design_system_root(&storage_ref)
            .expect("design root");
        let outside_file = outside.path().join("preview.json");
        std::fs::write(&outside_file, "{\"ok\":true}").expect("outside json");

        let result = paths.read_json_file::<serde_json::Value>(&root, &outside_file);

        assert!(result.is_err());
    }

    #[test]
    fn read_json_file_under_design_storage_root_rejects_outside_paths() {
        let temp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        let paths = DesignStoragePaths::new(temp.path()).expect("storage paths");
        let storage_ref = paths
            .storage_ref_for_design_system(&DesignSystemId::from_string("design-1".to_string()));
        let root = paths
            .ensure_design_system_root(&storage_ref)
            .expect("design root");
        let inside = paths
            .write_json_file(
                &root,
                "exports/package.json",
                &serde_json::json!({ "ok": true }),
            )
            .expect("inside json");
        let outside_file = outside.path().join("package.json");
        std::fs::write(&outside_file, "{\"ok\":true}").expect("outside json");

        let inside_value: serde_json::Value = paths
            .read_json_file_under_design_storage_root(&inside)
            .expect("read inside json");
        let outside_result =
            paths.read_json_file_under_design_storage_root::<serde_json::Value>(&outside_file);

        assert_eq!(inside_value["ok"].as_bool(), Some(true));
        assert!(outside_result.is_err());
    }
}
