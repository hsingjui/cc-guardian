//! Git operations module - refactored version
//!
//! This module provides git operations functionality through a modular architecture.
//! The implementation has been split into focused sub-modules for better maintainability.

use crate::error::{CheckpointError, Result as CcResult};
use chrono::DateTime;
use console::{Color, style};
use git2::{Commit, Delta, Oid, Repository, Signature};

// Sub-modules for organization
pub mod branch;
pub mod commit;
pub mod diff;
pub mod repository;
pub mod types;

// Re-export main types
pub use types::*;

/// Main GitOperations struct that coordinates all git operations
pub struct GitOperations {
    repo: Repository,
}

impl Clone for GitOperations {
    fn clone(&self) -> Self {
        // 重新打开同一个仓库
        let repo_path = self.repo.path();
        let repo = Repository::open(repo_path).expect("Failed to reopen repository");

        GitOperations { repo }
    }
}

impl GitOperations {
    /// Create a new GitOperations instance
    pub fn new(path: Option<&str>) -> CcResult<Self> {
        let repo_path = path.unwrap_or(".");
        let repo = match Repository::open(repo_path) {
            Ok(repo) => repo,
            Err(e) => match e.class() {
                git2::ErrorClass::Repository => {
                    // 如果不是Git仓库，尝试初始化
                    repository::RepositoryOperations::init_repository(repo_path)?
                }
                _ => return Err(CheckpointError::GitOperationFailed(e)),
            },
        };

        Ok(GitOperations { repo })
    }

    /// Create GitOperations from a path
    pub fn new_from_path<P: AsRef<std::path::Path>>(path: P) -> CcResult<Self> {
        let repo = Repository::open(path).map_err(|e| match e.class() {
            git2::ErrorClass::Repository => CheckpointError::RepositoryNotFound,
            _ => CheckpointError::GitOperationFailed(e),
        })?;
        Ok(GitOperations { repo })
    }

    /// Get reference to the underlying repository
    pub fn get_repo(&self) -> &Repository {
        &self.repo
    }

    /// Initialize checkpoints (create CCG branch)
    pub fn init_checkpoints(&self) -> CcResult<()> {
        self.create_or_get_checkpoints_branch()?;
        Ok(())
    }

    /// Create or get the CCG branch
    pub fn create_or_get_checkpoints_branch(&self) -> CcResult<git2::Branch> {
        // Try to get existing branch
        if let Ok(branch) = self
            .repo
            .find_branch(CCG_BRANCH_NAME, git2::BranchType::Local)
        {
            println!("🌿 检测到已存在的 '{CCG_BRANCH_NAME}' 分支");
            return Ok(branch);
        }

        // Check if we have HEAD commit
        let head_commit = match self.repo.head() {
            Ok(head) => head.peel_to_commit().ok(),
            Err(_) => None,
        };

        if let Some(commit) = head_commit {
            // Create branch based on current HEAD
            let branch = self
                .repo
                .branch(CCG_BRANCH_NAME, &commit, false)
                .map_err(CheckpointError::GitOperationFailed)?;
            println!("✅ '{CCG_BRANCH_NAME}' 分支创建成功");
            Ok(branch)
        } else {
            // Empty repository, create initial commit first
            println!("📝 空仓库检测到，创建初始提交...");
            let _commit_id = self.create_initial_commit()?;

            // Now try to get the branch
            self.repo
                .find_branch(CCG_BRANCH_NAME, git2::BranchType::Local)
                .map_err(CheckpointError::GitOperationFailed)
        }
    }

    /// Create a checkpoint (commit)
    pub fn create_checkpoint(&self, message: &str) -> CcResult<String> {
        let original_branch = self.ensure_ccg_branch()?;
        let result = self.create_commit_internal(message);
        self.restore_original_branch(&original_branch)?;
        result
    }

    /// Internal commit creation
    fn create_commit_internal(&self, message: &str) -> CcResult<String> {
        if !self.has_changes_to_commit()? {
            return Err(CheckpointError::NoChangesToCommit);
        }

        let signature = self.create_signature()?;
        let mut index = self
            .repo
            .index()
            .map_err(CheckpointError::GitOperationFailed)?;

        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(CheckpointError::GitOperationFailed)?;
        index.write().map_err(CheckpointError::GitOperationFailed)?;

        let tree_id = index
            .write_tree()
            .map_err(CheckpointError::GitOperationFailed)?;
        let parent_commit = self.get_parent_commit()?;
        let parents: Vec<&Commit> = parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();

        let commit_id = self
            .repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &self
                    .repo
                    .find_tree(tree_id)
                    .map_err(CheckpointError::GitOperationFailed)?,
                &parents,
            )
            .map_err(CheckpointError::GitOperationFailed)?;

        Ok(commit_id.to_string())
    }

    /// List checkpoints
    pub fn list_checkpoints(&self, limit: usize) -> CcResult<Vec<String>> {
        let mut revwalk = self
            .repo
            .revwalk()
            .map_err(CheckpointError::GitOperationFailed)?;
        revwalk
            .set_sorting(git2::Sort::TIME)
            .map_err(CheckpointError::GitOperationFailed)?;
        revwalk
            .push_head()
            .map_err(CheckpointError::GitOperationFailed)?;

        let mut commits = Vec::new();
        for (i, oid) in revwalk.enumerate() {
            if i >= limit {
                break;
            }

            let oid = oid.map_err(CheckpointError::GitOperationFailed)?;
            let commit = self
                .repo
                .find_commit(oid)
                .map_err(CheckpointError::GitOperationFailed)?;

            let short_hash = &oid.to_string()[..7];
            let message = commit
                .message()
                .unwrap_or("No commit message")
                .lines()
                .next()
                .unwrap_or("No commit message");
            let time = commit.time();
            let datetime = DateTime::from_timestamp(time.seconds(), 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown time".to_string());

            let final_message = message
                .strip_prefix("Checkpoint created with raw input: ")
                .unwrap_or(message);

            let formatted = format!(
                "{} {} {}",
                style(short_hash).fg(Color::Yellow).bold(),
                style(datetime).fg(Color::Cyan),
                style(final_message).fg(Color::White)
            );
            commits.push(formatted);
        }

        Ok(commits)
    }

    /// Find a commit by hash
    pub fn find_commit(&self, hash: &str) -> CcResult<Commit> {
        if let Ok(oid) = Oid::from_str(hash) {
            if let Ok(commit) = self.repo.find_commit(oid) {
                return Ok(commit);
            }
        }

        // Try short hash
        if hash.len() >= 2 && hash.len() < 40 {
            let mut revwalk = self
                .repo
                .revwalk()
                .map_err(CheckpointError::GitOperationFailed)?;
            revwalk
                .set_sorting(git2::Sort::TIME)
                .map_err(CheckpointError::GitOperationFailed)?;
            revwalk
                .push_head()
                .map_err(CheckpointError::GitOperationFailed)?;

            let mut matches = Vec::new();
            for oid_result in revwalk {
                let oid = oid_result.map_err(CheckpointError::GitOperationFailed)?;
                if oid.to_string().starts_with(hash) {
                    matches.push(oid);
                }
            }

            match matches.len() {
                0 => Err(CheckpointError::CheckpointNotFound(hash.to_string())),
                1 => self
                    .repo
                    .find_commit(matches[0])
                    .map_err(CheckpointError::GitOperationFailed),
                _ => Err(CheckpointError::InvalidHash(format!(
                    "短hash '{hash}' 匹配到多个提交"
                ))),
            }
        } else {
            Err(CheckpointError::InvalidHash(format!(
                "无效的hash格式: {hash}"
            )))
        }
    }

    /// Get commit details
    pub fn get_commit_details(&self, hash: &str) -> CcResult<String> {
        let commit = self.find_commit(hash)?;
        let full_hash = commit.id().to_string();
        let author = commit.author();
        let message = commit.message().unwrap_or("");
        let time = commit.time();

        let datetime = DateTime::from_timestamp(time.seconds(), 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown time".to_string());

        let result = format!(
            "{} {}\n{} {} <{}>\n{} {}\n\n{}\n{}\n",
            style("Commit:").fg(Color::White).bold(),
            style(&full_hash).fg(Color::Yellow).bold(),
            style("Author:").fg(Color::White).bold(),
            style(author.name().unwrap_or("Unknown")).fg(Color::Cyan),
            style(author.email().unwrap_or("unknown")).fg(Color::Cyan),
            style("Date:").fg(Color::White).bold(),
            style(&datetime).fg(Color::Green),
            style("Message:").fg(Color::White).bold(),
            style(message).fg(Color::White)
        );

        Ok(result)
    }

    /// Restore to a checkpoint
    pub fn restore_checkpoint(&self, hash: &str) -> CcResult<()> {
        let commit = self.find_commit(hash)?;
        let tree = commit.tree().map_err(CheckpointError::GitOperationFailed)?;

        if self.has_uncommitted_changes()? {
            return Err(CheckpointError::UncommittedChanges);
        }

        // 设置 checkout 选项以强制更新工作目录
        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        checkout_opts.force(); // 强制覆盖工作目录文件
        checkout_opts.remove_untracked(true); // 移除未跟踪的文件

        // 检出树到工作目录
        self.repo
            .checkout_tree(tree.as_object(), Some(&mut checkout_opts))
            .map_err(CheckpointError::GitOperationFailed)?;

        // 设置 HEAD 为分离状态指向目标提交
        self.repo
            .set_head_detached(commit.id())
            .map_err(CheckpointError::GitOperationFailed)?;

        Ok(())
    }

    /// 硬重置分支到指定检查点 - 真正的时光机效果
    pub fn reset_branch_to_checkpoint(&self, hash: &str) -> CcResult<()> {
        let commit = self.find_commit(hash)?;

        if self.has_uncommitted_changes()? {
            return Err(CheckpointError::UncommittedChanges);
        }

        // 获取当前分支引用
        let head = self
            .repo
            .head()
            .map_err(CheckpointError::GitOperationFailed)?;
        let branch_name = head.shorthand().unwrap_or("ccg");

        // 强制重置分支到目标提交
        let mut branch = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)
            .map_err(CheckpointError::GitOperationFailed)?;

        // 设置分支指向目标提交
        let reference = branch.get_mut();
        reference
            .set_target(commit.id(), "Reset branch to checkpoint")
            .map_err(CheckpointError::GitOperationFailed)?;

        // 硬重置工作目录和索引到目标提交
        self.repo
            .reset(commit.as_object(), git2::ResetType::Hard, None)
            .map_err(CheckpointError::GitOperationFailed)?;

        Ok(())
    }

    /// 获取当前 HEAD 提交
    pub fn get_head_commit(&self) -> CcResult<git2::Commit> {
        let head = self
            .repo
            .head()
            .map_err(CheckpointError::GitOperationFailed)?;
        let commit = head
            .peel_to_commit()
            .map_err(CheckpointError::GitOperationFailed)?;
        Ok(commit)
    }

    /// 计算两个提交之间的提交数量
    pub fn count_commits_between(&self, from_hash: &str, to_hash: &str) -> CcResult<usize> {
        let from_commit = self.find_commit(from_hash)?;
        let to_commit = self.find_commit(to_hash)?;

        // 使用 git2 的 revwalk 来计算提交数量
        let mut revwalk = self
            .repo
            .revwalk()
            .map_err(CheckpointError::GitOperationFailed)?;
        revwalk
            .push(to_commit.id())
            .map_err(CheckpointError::GitOperationFailed)?;
        revwalk
            .hide(from_commit.id())
            .map_err(CheckpointError::GitOperationFailed)?;

        let count = revwalk.count();
        Ok(count)
    }

    /// Get current branch name
    pub fn get_current_branch_name(&self) -> CcResult<String> {
        match self.repo.head() {
            Ok(head) => {
                let head_name = head.name().ok_or_else(|| {
                    CheckpointError::GitOperationFailed(git2::Error::from_str("HEAD has no name"))
                })?;

                if head_name == "HEAD" {
                    return Ok("HEAD".to_string());
                }

                Ok(head_name
                    .strip_prefix("refs/heads/")
                    .unwrap_or(head_name)
                    .to_string())
            }
            Err(e) => {
                if e.code() == git2::ErrorCode::UnbornBranch {
                    Ok("main".to_string())
                } else {
                    Err(CheckpointError::GitOperationFailed(e))
                }
            }
        }
    }

    /// Check if HEAD is detached
    pub fn is_head_detached(&self) -> CcResult<bool> {
        match self.repo.head() {
            Ok(head) => Ok(head.name().is_none_or(|name| name == "HEAD")),
            Err(e) => {
                if e.code() == git2::ErrorCode::UnbornBranch {
                    Ok(false)
                } else {
                    Err(CheckpointError::GitOperationFailed(e))
                }
            }
        }
    }

    /// Check if there are uncommitted changes
    pub fn has_uncommitted_changes(&self) -> CcResult<bool> {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true);
        let statuses = self
            .repo
            .statuses(Some(&mut opts))
            .map_err(CheckpointError::GitOperationFailed)?;
        Ok(!statuses.is_empty())
    }

    /// Create initial commit
    pub fn create_initial_commit(&self) -> CcResult<String> {
        println!("📝 创建初始提交...");

        let signature = self.create_signature()?;
        let mut index = self
            .repo
            .index()
            .map_err(CheckpointError::GitOperationFailed)?;

        if !self.has_non_ignored_files()? {
            println!("📝 没有文件可添加，创建空的初始提交...");
        } else {
            index
                .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
                .map_err(CheckpointError::GitOperationFailed)?;
            index.write().map_err(CheckpointError::GitOperationFailed)?;
        }

        let tree_id = index
            .write_tree()
            .map_err(CheckpointError::GitOperationFailed)?;
        let tree = self
            .repo
            .find_tree(tree_id)
            .map_err(CheckpointError::GitOperationFailed)?;

        let commit_id = self
            .repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                DEFAULT_COMMIT_MESSAGE,
                &tree,
                &[],
            )
            .map_err(CheckpointError::GitOperationFailed)?;

        println!("✅ 初始提交创建成功: {commit_id}");
        Ok(commit_id.to_string())
    }

    /// Show checkpoint with optional diff
    pub fn show_checkpoint(&self, hash: &str, show_diff: bool) -> CcResult<String> {
        let commit = self.find_commit(hash)?;
        let mut result = self.get_commit_details(hash)?;

        // 添加文件变更信息
        let diff_ops = diff::DiffOperations::new(&self.repo);
        if let Ok(diff) = diff_ops.get_commit_diff(&commit) {
            let mut stats = (0, 0, 0); // (added, modified, deleted)
            let mut files = Vec::new();

            for delta in diff.deltas() {
                if let Some(file) = delta.new_file().path() {
                    let status = delta.status();
                    let (status_str, color) = match status {
                        Delta::Added => {
                            stats.0 += 1;
                            ("A", Color::Green)
                        }
                        Delta::Deleted => {
                            stats.2 += 1;
                            ("D", Color::Red)
                        }
                        Delta::Modified => {
                            stats.1 += 1;
                            ("M", Color::Yellow)
                        }
                        Delta::Renamed => ("R", Color::Blue),
                        Delta::Copied => ("C", Color::Magenta),
                        _ => ("?", Color::White),
                    };

                    files.push(format!(
                        "  {} {}",
                        style(status_str).fg(color).bold(),
                        style(file.display()).fg(Color::White)
                    ));
                }
            }

            if !files.is_empty() {
                result.push_str(&format!(
                    "\n{} {} files changed",
                    style("Files:").fg(Color::White).bold(),
                    style(files.len()).fg(Color::Cyan).bold()
                ));

                if stats.0 > 0 || stats.1 > 0 || stats.2 > 0 {
                    result.push_str(" (");
                    let mut parts = Vec::new();
                    if stats.0 > 0 {
                        parts.push(format!(
                            "{} {}",
                            style(stats.0).fg(Color::Green).bold(),
                            style("added").fg(Color::Green)
                        ));
                    }
                    if stats.1 > 0 {
                        parts.push(format!(
                            "{} {}",
                            style(stats.1).fg(Color::Yellow).bold(),
                            style("modified").fg(Color::Yellow)
                        ));
                    }
                    if stats.2 > 0 {
                        parts.push(format!(
                            "{} {}",
                            style(stats.2).fg(Color::Red).bold(),
                            style("deleted").fg(Color::Red)
                        ));
                    }
                    result.push_str(&parts.join(", "));
                    result.push(')');
                }
                result.push_str("\n\n");

                for file in files {
                    result.push_str(&format!("{file}\n"));
                }
            }
        }

        if show_diff {
            result.push('\n');
            result.push_str(&format!(
                "{}\n",
                style("Detailed Diff:").fg(Color::White).bold()
            ));
            result.push_str(&diff_ops.get_commit_diff_content(hash)?);
        }

        Ok(result)
    }

    /// Ensure we're on CCG branch and return original branch
    pub fn ensure_ccg_branch(&self) -> CcResult<String> {
        let current_branch = self.get_current_branch_name()?;

        if current_branch != CCG_BRANCH_NAME {
            println!(
                "{} {} {} {} {}",
                style("🔄").fg(Color::Blue),
                style("切换到").fg(Color::White),
                style(CCG_BRANCH_NAME).fg(Color::Yellow).bold(),
                style("分支执行操作，当前分支:").fg(Color::White),
                style(&current_branch).fg(Color::Cyan)
            );

            let branch = self
                .repo
                .find_branch(CCG_BRANCH_NAME, git2::BranchType::Local)
                .map_err(CheckpointError::GitOperationFailed)?;
            let branch_ref = branch.get();
            self.repo
                .set_head(branch_ref.name().unwrap())
                .map_err(CheckpointError::GitOperationFailed)?;
        }

        Ok(current_branch)
    }

    /// Restore to original branch
    pub fn restore_original_branch(&self, original_branch: &str) -> CcResult<()> {
        if original_branch != CCG_BRANCH_NAME {
            let branch_ref = format!("refs/heads/{original_branch}");
            if let Err(e) = self.repo.set_head(&branch_ref) {
                println!(
                    "{} {} {}",
                    style("⚠️").fg(Color::Yellow),
                    style("警告: 无法切回原始分支").fg(Color::Yellow),
                    style(original_branch).fg(Color::Cyan)
                );
                return Err(CheckpointError::GitOperationFailed(e));
            } else {
                println!(
                    "{} {} {}",
                    style("🔄").fg(Color::Blue),
                    style("已切回原始分支:").fg(Color::White),
                    style(original_branch).fg(Color::Cyan)
                );
            }
        }
        Ok(())
    }

    /// Diff checkpoints
    pub fn diff_checkpoints(&self, hash_a: &str, hash_b: Option<&str>) -> CcResult<String> {
        let diff_ops = diff::DiffOperations::new(&self.repo);
        diff_ops.diff_commits(hash_a, hash_b)
    }

    /// Get working directory diff
    pub fn get_workdir_diff(&self) -> CcResult<git2::Diff> {
        let head = self.repo.head()?;
        let head_commit = head.peel_to_commit()?;
        let head_tree = head_commit.tree()?;

        self.repo
            .diff_tree_to_index(Some(&head_tree), None, None)
            .map_err(CheckpointError::GitOperationFailed)
    }

    /// Get commit diff content
    pub fn get_commit_diff_content(&self, hash: &str) -> CcResult<String> {
        let diff_ops = diff::DiffOperations::new(&self.repo);
        diff_ops.get_commit_diff_content(hash)
    }

    /// Prune checkpoints (placeholder implementation)
    pub fn prune_checkpoints(&self, _keep: Option<usize>, _before: Option<&str>) -> CcResult<()> {
        Ok(())
    }

    // Helper methods
    fn create_signature(&self) -> CcResult<Signature> {
        let config = self
            .repo
            .config()
            .map_err(CheckpointError::GitOperationFailed)?;
        let name = config.get_str("user.name").unwrap_or("Claude Checkpoint");
        let email = config
            .get_str("user.email")
            .unwrap_or("claude@checkpoint.local");
        Signature::now(name, email).map_err(CheckpointError::GitOperationFailed)
    }

    fn get_parent_commit(&self) -> CcResult<Option<Commit>> {
        let head = self
            .repo
            .head()
            .map_err(CheckpointError::GitOperationFailed)?;
        Ok(head.peel_to_commit().ok())
    }

    fn has_changes_to_commit(&self) -> CcResult<bool> {
        let parent_commit = match self.get_parent_commit()? {
            Some(commit) => commit,
            None => return self.has_non_ignored_files(),
        };

        let parent_tree = parent_commit
            .tree()
            .map_err(CheckpointError::GitOperationFailed)?;
        let mut temp_index = self
            .repo
            .index()
            .map_err(CheckpointError::GitOperationFailed)?;

        temp_index
            .clear()
            .map_err(CheckpointError::GitOperationFailed)?;
        temp_index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(CheckpointError::GitOperationFailed)?;

        let temp_tree_id = temp_index
            .write_tree()
            .map_err(CheckpointError::GitOperationFailed)?;
        let temp_tree = self
            .repo
            .find_tree(temp_tree_id)
            .map_err(CheckpointError::GitOperationFailed)?;

        let diff = self
            .repo
            .diff_tree_to_tree(Some(&parent_tree), Some(&temp_tree), None)
            .map_err(CheckpointError::GitOperationFailed)?;

        Ok(diff.deltas().len() > 0)
    }

    fn has_non_ignored_files(&self) -> CcResult<bool> {
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true);
        opts.include_ignored(false);

        let statuses = self
            .repo
            .statuses(Some(&mut opts))
            .map_err(CheckpointError::GitOperationFailed)?;
        Ok(!statuses.is_empty())
    }
}
