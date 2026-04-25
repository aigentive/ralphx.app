use std::collections::{BTreeMap, HashSet};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::domain::entities::ProjectId;
use crate::error::{AppError, AppResult};

const IGNORED_SOURCE_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".turbo",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "tmp",
];
const MAX_MANIFEST_FILES: usize = 5_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignSourceManifestEntry {
    pub relative_path: String,
    pub sha256: String,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignSourceManifest {
    pub project_id: ProjectId,
    pub entries: Vec<DesignSourceManifestEntry>,
}

impl DesignSourceManifest {
    pub fn source_hashes(&self) -> BTreeMap<String, String> {
        self.entries
            .iter()
            .map(|entry| (entry.relative_path.clone(), entry.sha256.clone()))
            .collect()
    }
}

pub fn build_design_source_manifest(
    project_id: ProjectId,
    project_root: impl AsRef<Path>,
    selected_paths: &[String],
) -> AppResult<DesignSourceManifest> {
    let project_root = canonical_project_root(project_root.as_ref())?;
    let selected_roots = resolve_selected_roots(&project_root, selected_paths)?;
    let mut entries = Vec::new();
    let mut seen_files = HashSet::new();

    for selected_root in selected_roots {
        collect_manifest_entries(&project_root, &selected_root, &mut seen_files, &mut entries)?;
    }

    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(DesignSourceManifest {
        project_id,
        entries,
    })
}

fn canonical_project_root(project_root: &Path) -> AppResult<PathBuf> {
    if project_root.as_os_str().is_empty() {
        return Err(AppError::Validation(
            "Design source project root cannot be empty".to_string(),
        ));
    }
    if !project_root.is_absolute() {
        return Err(AppError::Validation(
            "Design source project root must be absolute".to_string(),
        ));
    }

    let canonical = project_root.canonicalize().map_err(|error| {
        AppError::Validation(format!(
            "Failed to canonicalize design source project root: {error}"
        ))
    })?;
    if !canonical.is_dir() {
        return Err(AppError::Validation(
            "Design source project root must be a directory".to_string(),
        ));
    }
    Ok(canonical)
}

fn resolve_selected_roots(
    canonical_project_root: &Path,
    selected_paths: &[String],
) -> AppResult<Vec<PathBuf>> {
    if selected_paths.is_empty() {
        return Ok(vec![canonical_project_root.to_path_buf()]);
    }

    let mut roots = Vec::new();
    let mut seen = HashSet::new();
    for raw_path in selected_paths {
        let Some(relative_path) = validated_relative_source_path(raw_path)? else {
            continue;
        };
        let candidate = canonical_project_root.join(relative_path);
        let canonical = candidate.canonicalize().map_err(|error| {
            AppError::Validation(format!(
                "Failed to canonicalize selected design source path: {error}"
            ))
        })?;
        ensure_under(
            &canonical,
            canonical_project_root,
            "selected design source path",
        )?;
        if seen.insert(canonical.clone()) {
            roots.push(canonical);
        }
    }

    if roots.is_empty() {
        roots.push(canonical_project_root.to_path_buf());
    }
    roots.sort();
    Ok(roots)
}

fn validated_relative_source_path(raw_path: &str) -> AppResult<Option<PathBuf>> {
    let raw_path = raw_path.trim();
    if raw_path.is_empty() {
        return Ok(None);
    }

    let path = Path::new(raw_path);
    if path.is_absolute() {
        return Err(AppError::Validation(
            "Design source paths must be relative to the source project".to_string(),
        ));
    }

    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => safe.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::Validation(
                    "Design source paths cannot contain parent, root, or prefix components"
                        .to_string(),
                ));
            }
        }
    }

    if safe.as_os_str().is_empty() {
        return Ok(None);
    }
    Ok(Some(safe))
}

fn collect_manifest_entries(
    canonical_project_root: &Path,
    candidate: &Path,
    seen_files: &mut HashSet<PathBuf>,
    entries: &mut Vec<DesignSourceManifestEntry>,
) -> AppResult<()> {
    let canonical = candidate.canonicalize().map_err(|error| {
        AppError::Validation(format!(
            "Failed to canonicalize design source candidate: {error}"
        ))
    })?;
    ensure_under(
        &canonical,
        canonical_project_root,
        "design source candidate",
    )?;

    let metadata = std::fs::metadata(&canonical).map_err(|error| {
        AppError::Infrastructure(format!("Failed to read design source metadata: {error}"))
    })?;

    if metadata.is_dir() {
        if is_ignored_dir(&canonical) {
            return Ok(());
        }
        let mut children = Vec::new();
        for entry in std::fs::read_dir(&canonical).map_err(|error| {
            AppError::Infrastructure(format!("Failed to read design source directory: {error}"))
        })? {
            let entry = entry.map_err(|error| {
                AppError::Infrastructure(format!("Failed to inspect design source entry: {error}"))
            })?;
            let child = entry.path().canonicalize().map_err(|error| {
                AppError::Validation(format!(
                    "Failed to canonicalize design source child: {error}"
                ))
            })?;
            ensure_under(&child, canonical_project_root, "design source child")?;
            children.push(child);
        }
        children.sort();
        for child in children {
            collect_manifest_entries(canonical_project_root, &child, seen_files, entries)?;
        }
        return Ok(());
    }

    if !metadata.is_file() || !seen_files.insert(canonical.clone()) {
        return Ok(());
    }
    if entries.len() >= MAX_MANIFEST_FILES {
        return Err(AppError::Validation(format!(
            "Design source manifest exceeds {MAX_MANIFEST_FILES} files"
        )));
    }

    let relative_path = canonical
        .strip_prefix(canonical_project_root)
        .map_err(|_| AppError::Validation("Design source file escaped project root".to_string()))
        .map(relative_path_string)?;
    let sha256 = hash_file(&canonical)?;

    entries.push(DesignSourceManifestEntry {
        relative_path,
        sha256,
        byte_len: metadata.len(),
    });
    Ok(())
}

fn is_ignored_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| IGNORED_SOURCE_DIRS.contains(&name))
        .unwrap_or(false)
}

fn hash_file(path: &Path) -> AppResult<String> {
    let mut file = std::fs::File::open(path).map_err(|error| {
        AppError::Infrastructure(format!("Failed to open design source file: {error}"))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(|error| {
            AppError::Infrastructure(format!("Failed to read design source file: {error}"))
        })?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn relative_path_string(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn ensure_under(path: &Path, root: &Path, label: &str) -> AppResult<()> {
    if path.starts_with(root) {
        return Ok(());
    }
    Err(AppError::Validation(format!(
        "{label} escaped design source project root"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_project_file(root: &Path, relative_path: &str, content: &str) {
        let path = safe_test_child(root, relative_path);
        let parent = path.parent().expect("test path parent");
        std::fs::create_dir_all(parent).expect("create parent");
        std::fs::write(path, content).expect("write file");
    }

    fn safe_test_child(root: &Path, relative_path: &str) -> PathBuf {
        let relative_path = validated_relative_source_path(relative_path)
            .expect("valid relative path")
            .expect("non-empty relative path");
        root.join(relative_path)
    }

    #[test]
    fn manifest_hashes_selected_files_deterministically() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_project_file(temp.path(), "src/App.tsx", "export function App() {}\n");
        write_project_file(temp.path(), "src/styles.css", ".app { color: red; }\n");
        write_project_file(temp.path(), "node_modules/pkg/index.js", "ignored\n");

        let manifest = build_design_source_manifest(
            ProjectId::from_string("project-1".to_string()),
            temp.path(),
            &["src".to_string(), "./src/App.tsx".to_string()],
        )
        .expect("manifest");

        assert_eq!(
            manifest
                .entries
                .iter()
                .map(|entry| entry.relative_path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/App.tsx", "src/styles.css"]
        );
        assert_eq!(manifest.entries[0].sha256.len(), 64);
        assert_eq!(manifest.source_hashes().len(), 2);
    }

    #[test]
    fn manifest_rejects_parent_and_absolute_selected_paths() {
        let temp = tempfile::tempdir().expect("tempdir");
        write_project_file(temp.path(), "src/App.tsx", "content\n");

        let parent_result = build_design_source_manifest(
            ProjectId::from_string("project-1".to_string()),
            temp.path(),
            &["../outside".to_string()],
        );
        assert!(parent_result.is_err());

        let absolute_result = build_design_source_manifest(
            ProjectId::from_string("project-1".to_string()),
            temp.path(),
            &[temp
                .path()
                .join("src/App.tsx")
                .to_string_lossy()
                .to_string()],
        );
        assert!(absolute_result.is_err());
    }

    #[test]
    fn manifest_rejects_relative_project_root() {
        let result = build_design_source_manifest(
            ProjectId::from_string("project-1".to_string()),
            Path::new("relative/project"),
            &[],
        );

        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn manifest_rejects_symlink_escape() {
        let project = tempfile::tempdir().expect("project tempdir");
        let outside = tempfile::tempdir().expect("outside tempdir");
        write_project_file(outside.path(), "secret.txt", "outside\n");
        let link_path = safe_test_child(project.path(), "linked-secret.txt");
        std::os::unix::fs::symlink(safe_test_child(outside.path(), "secret.txt"), link_path)
            .expect("symlink");

        let result = build_design_source_manifest(
            ProjectId::from_string("project-1".to_string()),
            project.path(),
            &["linked-secret.txt".to_string()],
        );

        assert!(result.is_err());
    }
}
