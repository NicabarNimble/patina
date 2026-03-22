use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitHost {
    repo_root: PathBuf,
}

impl GitHost {
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
        }
    }

    pub fn create_tag(&self, name: &str) -> Result<()> {
        self.run_git(["tag", name])?;
        Ok(())
    }

    pub fn delete_tag(&self, name: &str) -> Result<()> {
        self.run_git(["tag", "-d", name])?;
        Ok(())
    }

    pub fn tag_exists(&self, name: &str) -> Result<bool> {
        let out = self.run_git_capture(["tag", "--list", name])?;
        Ok(out.lines().any(|line| line.trim() == name))
    }

    pub fn commit(&self, message: &str) -> Result<String> {
        self.run_git(["add", "-A"])?;
        self.run_git(["commit", "-m", message])?;
        self.run_git_capture(["rev-parse", "HEAD"])
            .map(|s| s.trim().to_string())
    }

    pub fn log_oneline(&self, limit: u32) -> Result<Vec<String>> {
        let out = self.run_git_capture(["log", "--oneline", "--max-count", &limit.to_string()])?;
        Ok(out.lines().map(|s| s.to_string()).collect())
    }

    pub fn diff_stat(&self) -> Result<String> {
        self.run_git_capture(["diff", "--stat"])
    }

    fn run_git<const N: usize>(&self, args: [&str; N]) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.repo_root)
            .args(args)
            .output()
            .with_context(|| "running git command")?;
        if output.status.success() {
            Ok(())
        } else {
            anyhow::bail!(
                "git command failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
    }

    fn run_git_capture<const N: usize>(&self, args: [&str; N]) -> Result<String> {
        let output = Command::new("git")
            .current_dir(&self.repo_root)
            .args(args)
            .output()
            .with_context(|| "running git command")?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            anyhow::bail!(
                "git command failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        Command::new("git")
            .current_dir(root)
            .args(["init"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(root)
            .args(["config", "user.name", "Test User"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(root)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .unwrap();
        std::fs::write(root.join("README.md"), "init\n").unwrap();
        Command::new("git")
            .current_dir(root)
            .args(["add", "README.md"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(root)
            .args(["commit", "-m", "init"])
            .status()
            .unwrap();
        tmp
    }

    #[test]
    fn git_tag_and_log_flow() {
        let tmp = init_repo();
        let host = GitHost::new(tmp.path());
        host.create_tag("v0.1.0").unwrap();
        assert!(host.tag_exists("v0.1.0").unwrap());
        let log = host.log_oneline(1).unwrap();
        assert_eq!(log.len(), 1);
        host.delete_tag("v0.1.0").unwrap();
        assert!(!host.tag_exists("v0.1.0").unwrap());
    }

    #[test]
    fn git_commit_and_diff_stat() {
        let tmp = init_repo();
        let host = GitHost::new(tmp.path());
        std::fs::write(tmp.path().join("notes.txt"), "hello\n").unwrap();
        let stat = host.diff_stat().unwrap();
        assert!(stat.contains("notes.txt") || stat.is_empty());
        let sha = host.commit("add notes").unwrap();
        assert!(!sha.is_empty());
    }
}
