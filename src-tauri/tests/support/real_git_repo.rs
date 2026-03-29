pub struct RealGitRepo {
    pub dir: tempfile::TempDir,
    pub task_branch: String,
}

impl RealGitRepo {
    #[allow(dead_code)]
    pub fn path(&self) -> &std::path::Path {
        self.dir.path()
    }

    #[allow(dead_code)]
    pub fn path_string(&self) -> String {
        self.dir.path().to_string_lossy().to_string()
    }
}

pub fn setup_real_git_repo() -> RealGitRepo {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let path = dir.path();

    let _ = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(path)
        .output()
        .expect("git init");

    let _ = std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output();

    std::fs::write(path.join("README.md"), "# test repo").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(path)
        .output();

    let task_branch = "task/test-task-branch".to_string();
    let _ = std::process::Command::new("git")
        .args(["checkout", "-b", &task_branch])
        .current_dir(path)
        .output();
    std::fs::write(path.join("feature.rs"), "// feature code\nfn feature() {}").unwrap();
    let _ = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "add feature"])
        .current_dir(path)
        .output();

    let _ = std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(path)
        .output();

    RealGitRepo { dir, task_branch }
}
