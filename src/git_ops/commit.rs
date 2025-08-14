//! Commit creation and management operations

use crate::error::{CheckpointError, Result as CcResult};
use chrono::{DateTime, Utc};
use console::{Color, style};
use git2::{Commit, Oid, Repository, Signature, Tree};

/// Operations related to commit management
pub struct CommitOperations<'a> {
    repo: &'a Repository,
}

impl<'a> CommitOperations<'a> {
    /// Create a new CommitOperations instance
    pub fn new(repo: &'a Repository) -> Self {
        Self { repo }
    }

    /// Create a signature for commits
    pub fn create_signature(&self) -> CcResult<Signature> {
        let _now = Utc::now();

        // 尝试获取 Git 配置中的用户信息
        let config = self
            .repo
            .config()
            .map_err(CheckpointError::GitOperationFailed)?;
        let name = config
            .get_str("user.name")
            .unwrap_or("Claude Code Checkpoint");
        let email = config
            .get_str("user.email")
            .unwrap_or("claudecode@checkpoint.local");

        Signature::now(name, email).map_err(CheckpointError::GitOperationFailed)
    }

    /// Get the parent commit (HEAD)
    pub fn get_parent_commit(&self) -> CcResult<Option<Commit>> {
        let head = self
            .repo
            .head()
            .map_err(CheckpointError::GitOperationFailed)?;
        let head_commit = head.peel_to_commit().ok();
        Ok(head_commit)
    }

    /// Check if there are changes to commit
    pub fn has_changes_to_commit(&self) -> CcResult<bool> {
        // 获取父提交作为比较基准
        let parent_commit = match self.get_parent_commit()? {
            Some(commit) => commit,
            None => {
                // 没有父提交（初始状态），检查是否有非忽略的文件
                return self.has_non_ignored_files();
            }
        };

        // 比较工作目录与父提交的差异
        let parent_tree = parent_commit
            .tree()
            .map_err(CheckpointError::GitOperationFailed)?;

        // 创建一个临时索引，包含工作目录的所有变更
        let mut temp_index = self
            .repo
            .index()
            .map_err(CheckpointError::GitOperationFailed)?;

        // 清空临时索引并添加所有文件（这样可以检测到所有变更，包括新文件、修改和删除）
        temp_index
            .clear()
            .map_err(CheckpointError::GitOperationFailed)?;
        temp_index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(CheckpointError::GitOperationFailed)?;

        // 写入临时树对象
        let temp_tree_id = temp_index
            .write_tree()
            .map_err(CheckpointError::GitOperationFailed)?;
        let temp_tree = self
            .repo
            .find_tree(temp_tree_id)
            .map_err(CheckpointError::GitOperationFailed)?;

        // 比较父提交的树与临时树的差异
        let diff = self
            .repo
            .diff_tree_to_tree(Some(&parent_tree), Some(&temp_tree), None)
            .map_err(CheckpointError::GitOperationFailed)?;

        // 检查是否有变更
        Ok(diff.deltas().len() > 0)
    }

    /// Check if there are non-ignored files in the working directory
    pub fn has_non_ignored_files(&self) -> CcResult<bool> {
        // 检查工作目录中是否有非忽略的文件
        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true);
        opts.include_ignored(false);

        let statuses = self
            .repo
            .statuses(Some(&mut opts))
            .map_err(CheckpointError::GitOperationFailed)?;
        Ok(!statuses.is_empty())
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

    /// Create a new commit (checkpoint)
    ///
    /// # Arguments
    /// * `message` - The commit message
    ///
    /// # Returns
    /// The commit ID as a string
    pub fn create_commit(&self, message: &str) -> CcResult<String> {
        // 检查是否有实际的文件变更
        if !self.has_changes_to_commit()? {
            return Err(CheckpointError::NoChangesToCommit);
        }

        let signature = self.create_signature()?;

        // 添加文件到索引并创建树
        let mut index = self
            .repo
            .index()
            .map_err(CheckpointError::GitOperationFailed)?;

        // 添加所有变更的文件到暂存区
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(CheckpointError::GitOperationFailed)?;

        // 写入索引到磁盘
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

        // 提交后，确保工作区与最新提交同步
        // 这会清理工作区，使其与刚创建的提交保持一致
        self.reset_to_head()?;

        Ok(commit_id.to_string())
    }

    /// Create an initial commit for a new repository
    ///
    /// # Returns
    /// The commit ID as a string
    pub fn create_initial_commit(&self) -> CcResult<String> {
        use super::types::DEFAULT_COMMIT_MESSAGE;

        println!("📝 创建初始提交...");

        let signature = self.create_signature()?;

        // 添加所有文件到索引
        let mut index = self
            .repo
            .index()
            .map_err(CheckpointError::GitOperationFailed)?;

        // 检查是否有文件可以添加
        if !self.has_non_ignored_files()? {
            // 如果没有文件，创建一个空的初始提交
            println!("📝 没有文件可添加，创建空的初始提交...");

            // 创建空树
            let tree_id = index
                .write_tree()
                .map_err(CheckpointError::GitOperationFailed)?;
            let tree = self
                .repo
                .find_tree(tree_id)
                .map_err(CheckpointError::GitOperationFailed)?;

            // 创建初始提交
            let commit_id = self
                .repo
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    DEFAULT_COMMIT_MESSAGE,
                    &tree,
                    &[], // 没有父提交
                )
                .map_err(CheckpointError::GitOperationFailed)?;

            println!("✅ 初始提交创建成功: {commit_id}");
            return Ok(commit_id.to_string());
        }

        // 添加所有文件到索引
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(CheckpointError::GitOperationFailed)?;
        index.write().map_err(CheckpointError::GitOperationFailed)?;

        // 写入树对象
        let tree_id = index
            .write_tree()
            .map_err(CheckpointError::GitOperationFailed)?;
        let tree = self
            .repo
            .find_tree(tree_id)
            .map_err(CheckpointError::GitOperationFailed)?;

        // 创建初始提交
        let commit_id = self
            .repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                DEFAULT_COMMIT_MESSAGE,
                &tree,
                &[], // 没有父提交
            )
            .map_err(CheckpointError::GitOperationFailed)?;

        println!("✅ 初始提交创建成功: {commit_id}");

        Ok(commit_id.to_string())
    }

    /// Reset working directory to HEAD commit
    ///
    /// This ensures the working directory is clean after a commit.
    pub fn reset_to_head(&self) -> CcResult<()> {
        let head = self
            .repo
            .head()
            .map_err(CheckpointError::GitOperationFailed)?;
        let head_commit = head
            .peel_to_commit()
            .map_err(CheckpointError::GitOperationFailed)?;

        // 使用 HARD 重置，这会重置索引和工作区
        self.repo
            .reset(head_commit.as_object(), git2::ResetType::Hard, None)
            .map_err(CheckpointError::GitOperationFailed)?;

        Ok(())
    }

    /// Find a commit by hash (supports both full and short hashes)
    ///
    /// # Arguments
    /// * `hash` - Full or partial commit hash
    ///
    /// # Returns
    /// The found commit
    pub fn find_commit(&self, hash: &str) -> CcResult<Commit> {
        // 首先尝试完整的hash
        if let Ok(oid) = Oid::from_str(hash) {
            if let Ok(commit) = self.repo.find_commit(oid) {
                return Ok(commit);
            }
        }

        // 如果完整hash失败，尝试短hash查询
        if hash.len() >= 2 && hash.len() < 40 {
            // 遍历所有提交，查找匹配的短hash
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
                let oid_str = oid.to_string();

                if oid_str.starts_with(hash) {
                    matches.push(oid);
                }
            }

            match matches.len() {
                0 => Err(CheckpointError::CheckpointNotFound(hash.to_string())),
                1 => {
                    let commit = self
                        .repo
                        .find_commit(matches[0])
                        .map_err(CheckpointError::GitOperationFailed)?;
                    Ok(commit)
                }
                _ => {
                    // 多个匹配，返回错误并提示用户
                    let mut error_msg = format!("短hash '{hash}' 匹配到多个提交:\n");
                    for (i, oid) in matches.iter().take(5).enumerate() {
                        if let Ok(commit) = self.repo.find_commit(*oid) {
                            let short_hash = &oid.to_string()[..7];
                            let message = commit
                                .message()
                                .unwrap_or("No message")
                                .lines()
                                .next()
                                .unwrap_or("No message");
                            error_msg.push_str(&format!("  {short_hash} - {message}\n"));
                        }
                        if i >= 4 && matches.len() > 5 {
                            error_msg
                                .push_str(&format!("  ... 还有 {} 个匹配\n", matches.len() - 5));
                            break;
                        }
                    }
                    error_msg.push_str("请使用更长的hash前缀来唯一标识提交");
                    Err(CheckpointError::InvalidHash(error_msg))
                }
            }
        } else if hash.len() < 2 {
            Err(CheckpointError::InvalidHash(format!(
                "hash太短，至少需要2个字符: {hash}"
            )))
        } else {
            Err(CheckpointError::InvalidHash(format!(
                "无效的hash格式: {hash}"
            )))
        }
    }

    /// Get detailed information about a commit
    ///
    /// # Arguments
    /// * `hash` - Commit hash (full or partial)
    ///
    /// # Returns
    /// Formatted string with commit details
    pub fn get_commit_details(&self, hash: &str) -> CcResult<String> {
        let commit = self.find_commit(hash)?;
        let full_hash = commit.id().to_string();

        let author = commit.author();
        let committer = commit.committer();
        let message = commit.message().unwrap_or("");
        let time = commit.time();

        let datetime = DateTime::from_timestamp(time.seconds(), 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown time".to_string());

        // 使用彩色输出格式化提交详情
        let mut result = String::new();

        // 提交hash - 黄色高亮
        result.push_str(&format!(
            "{} {}\n",
            style("Commit:").fg(Color::White).bold(),
            style(&full_hash).fg(Color::Yellow).bold()
        ));

        // 作者信息 - 青色
        result.push_str(&format!(
            "{} {} <{}>\n",
            style("Author:").fg(Color::White).bold(),
            style(author.name().unwrap_or("Unknown")).fg(Color::Cyan),
            style(author.email().unwrap_or("unknown")).fg(Color::Cyan)
        ));

        // 日期 - 绿色
        result.push_str(&format!(
            "{} {}\n",
            style("Date:").fg(Color::White).bold(),
            style(&datetime).fg(Color::Green)
        ));

        // 提交者信息（如果与作者不同）
        if author.name() != committer.name() || author.email() != committer.email() {
            result.push_str(&format!(
                "{} {} <{}>\n",
                style("Committer:").fg(Color::White).bold(),
                style(committer.name().unwrap_or("Unknown")).fg(Color::Cyan),
                style(committer.email().unwrap_or("unknown")).fg(Color::Cyan)
            ));
        }

        // 提交消息 - 白色
        result.push_str(&format!(
            "\n{}\n{}\n",
            style("Message:").fg(Color::White).bold(),
            style(message).fg(Color::White)
        ));

        Ok(result)
    }

    /// List commits with formatting
    ///
    /// # Arguments
    /// * `limit` - Maximum number of commits to return
    ///
    /// # Returns
    /// Vector of formatted commit strings
    pub fn list_commits(&self, limit: usize) -> CcResult<Vec<String>> {
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

            // 获取提交的短hash（前7位）
            let short_hash = &oid.to_string()[..7];

            // 获取提交信息（第一行）
            let message = commit
                .message()
                .unwrap_or("No commit message")
                .lines()
                .next()
                .unwrap_or("No commit message");

            // 获取提交时间
            let time = commit.time();
            let datetime = DateTime::from_timestamp(time.seconds(), 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown time".to_string());

            // 格式化输出：短hash + 时间 + 提交信息，添加颜色
            let formatted = format!(
                "{} {} {}",
                style(short_hash).fg(Color::Yellow).bold(),
                style(datetime).fg(Color::Cyan),
                style(message).fg(Color::White)
            );
            commits.push(formatted);
        }

        Ok(commits)
    }

    /// Restore (checkout) to a specific commit
    ///
    /// # Arguments
    /// * `hash` - Commit hash to restore to
    pub fn restore_commit(&self, hash: &str) -> CcResult<()> {
        let commit = self.find_commit(hash)?;
        let tree = commit.tree().map_err(CheckpointError::GitOperationFailed)?;

        // 检查是否有未提交的更改
        if self.has_uncommitted_changes()? {
            return Err(CheckpointError::UncommittedChanges);
        }

        // 执行 checkout 操作
        self.repo
            .checkout_tree(tree.as_object(), None)
            .map_err(CheckpointError::GitOperationFailed)?;

        // 更新 HEAD 指针
        self.repo
            .set_head_detached(commit.id())
            .map_err(CheckpointError::GitOperationFailed)?;

        Ok(())
    }

    /// Checkout a tree to the working directory
    ///
    /// # Arguments
    /// * `tree` - The tree to checkout
    pub fn checkout_tree(&self, tree: &Tree) -> CcResult<()> {
        let mut opts = git2::build::CheckoutBuilder::new();
        opts.force().safe();

        self.repo
            .checkout_tree(tree.as_object(), Some(&mut opts))
            .map_err(CheckpointError::GitOperationFailed)
    }
}
