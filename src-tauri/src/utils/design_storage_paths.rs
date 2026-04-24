use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::domain::entities::{DesignStorageRootRef, DesignSystemId};
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
}
