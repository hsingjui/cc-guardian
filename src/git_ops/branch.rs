//! Branch management operations

use super::types::CCG_BRANCH_NAME;
use crate::error::{CheckpointError, Result as CcResult};
use console::{Color, style};
use git2::{Branch, Repository};

/// Operations related to branch management
pub struct BranchOperations<'a> {
    repo: &'a Repository,
}

impl<'a> BranchOperations<'a> {
    /// Create a new BranchOperations instance
    pub fn new(repo: &'a Repository) -> Self {
        Self { repo }
    }

    /// Get the current branch name
    pub fn get_current_branch_name(&self) -> CcResult<String> {
        match self.repo.head() {
            Ok(head) => {
                let head_name = head.name().ok_or_else(|| {
                    CheckpointError::GitOperationFailed(git2::Error::from_str("HEAD has no name"))
                })?;

                if head_name == "HEAD" {
                    return Ok("HEAD".to_string());
                }

                // 移除 "refs/heads/" 前缀
                Ok(head_name
                    .strip_prefix("refs/heads/")
                    .unwrap_or(head_name)
                    .to_string())
            }
            Err(e) => {
                // 如果是 UnbornBranch 错误，说明仓库刚初始化，还没有提交
                if e.code() == git2::ErrorCode::UnbornBranch {
                    // 返回默认分支名称，通常是 main 或 master
                    let default_branch = self
                        .get_default_branch_name()
                        .unwrap_or_else(|| "main".to_string());
                    Ok(default_branch)
                } else {
                    Err(CheckpointError::GitOperationFailed(e))
                }
            }
        }
    }

    /// Switch to a specific branch
    pub fn switch_to_branch(&self, branch_name: &str) -> CcResult<()> {
        let branch_ref = format!("refs/heads/{branch_name}");
        self.repo
            .set_head(&branch_ref)
            .map_err(CheckpointError::GitOperationFailed)
    }

    /// Validate that a branch is in a good state
    pub fn validate_branch(&self, branch: &Branch) -> CcResult<()> {
        // 验证分支是否可以获取到引用
        let branch_ref = branch.get().name().ok_or_else(|| {
            CheckpointError::GitOperationFailed(git2::Error::from_str(
                "Branch has no reference name",
            ))
        })?;

        // 验证分支引用是否有效
        if let Err(e) = self.repo.find_reference(branch_ref) {
            return Err(CheckpointError::GitOperationFailed(git2::Error::from_str(
                &format!("Invalid branch reference: {e}"),
            )));
        }

        Ok(())
    }

    /// Get the default branch name from Git configuration
    fn get_default_branch_name(&self) -> Option<String> {
        // 尝试从 Git 配置获取默认分支名称
        if let Ok(config) = self.repo.config() {
            if let Ok(branch_name) = config.get_str("init.defaultBranch") {
                return Some(branch_name.to_string());
            }
        }

        // 如果没有配置，返回 None，调用者会使用默认值
        None
    }

    /// Create or get the CCG branch
    ///
    /// This method will either find an existing CCG branch or create a new one.
    /// It handles various edge cases including empty repositories and missing branches.
    pub fn create_or_get_ccg_branch(&self) -> CcResult<Branch> {
        // 尝试获取已存在的分支
        if let Ok(branch) = self
            .repo
            .find_branch(CCG_BRANCH_NAME, git2::BranchType::Local)
        {
            // 分支已存在，验证并准备
            println!("🌿 检测到已存在的 '{CCG_BRANCH_NAME}' 分支");
            self.ensure_ccg_branch_ready(&branch)?;
            return Ok(branch);
        }

        // 检查当前分支是否就是ccg分支（可能由于某种原因find_branch没有找到）
        let current_branch_name = self.get_current_branch_name()?;
        if current_branch_name == CCG_BRANCH_NAME {
            println!("🌿 当前已在 '{CCG_BRANCH_NAME}' 分支上");
            // 尝试重新获取分支
            if let Ok(branch) = self
                .repo
                .find_branch(CCG_BRANCH_NAME, git2::BranchType::Local)
            {
                self.ensure_ccg_branch_ready(&branch)?;
                return Ok(branch);
            }
        }

        // 如果分支不存在，创建新分支
        println!("🌿 创建 '{CCG_BRANCH_NAME}' 分支...");

        // 首先检查是否有 HEAD 提交
        let head_commit = match self.repo.head() {
            Ok(head) => head.peel_to_commit().ok(),
            Err(_) => None,
        };

        if let Some(commit) = head_commit {
            // 有提交，基于当前 HEAD 创建分支
            let branch = self
                .repo
                .branch(CCG_BRANCH_NAME, &commit, false)
                .map_err(CheckpointError::GitOperationFailed)?;
            println!("✅ '{CCG_BRANCH_NAME}' 分支创建成功");

            // 切换到新创建的分支
            let branch_ref = branch.get();
            self.repo
                .set_head(branch_ref.name().unwrap())
                .map_err(CheckpointError::GitOperationFailed)?;
            println!("🔄 已切换到 '{CCG_BRANCH_NAME}' 分支");

            Ok(branch)
        } else {
            // 没有提交，这是一个空的仓库
            // 检查HEAD是否已经指向ccg分支
            let head_ref_name = match self.repo.head() {
                Ok(head) => head.name().map(|s| s.to_string()),
                Err(_) => None,
            };

            if head_ref_name == Some(format!("refs/heads/{CCG_BRANCH_NAME}")) {
                // HEAD已经指向ccg分支，需要创建初始提交
                // 注意：这里我们需要调用 CommitOperations，但为了避免循环依赖，
                // 我们将在更高层次的 GitOperations 中处理这个逻辑
                println!(
                    "📝 空仓库检测到，HEAD已指向 '{CCG_BRANCH_NAME}' 分支，需要创建初始提交..."
                );

                // 返回一个特殊错误，让调用者知道需要创建初始提交
                Err(CheckpointError::GitOperationFailed(git2::Error::from_str(
                    "Empty repository detected, initial commit needed",
                )))
            } else {
                // HEAD不指向ccg分支，也需要创建初始提交
                println!("📝 空仓库检测到，需要创建初始提交...");

                // 返回一个特殊错误，让调用者知道需要创建初始提交
                Err(CheckpointError::GitOperationFailed(git2::Error::from_str(
                    "Empty repository detected, initial commit needed",
                )))
            }
        }
    }

    /// Ensure CCG branch is ready for use
    ///
    /// This method validates the branch and ensures it has at least one commit.
    pub fn ensure_ccg_branch_ready(&self, branch: &Branch) -> CcResult<()> {
        // 验证分支有效性
        self.validate_branch(branch)?;

        // 切换到ccg分支
        self.switch_to_ccg_branch()?;
        println!("🔄 已切换到 '{CCG_BRANCH_NAME}' 分支");

        // 检查分支是否有提交
        let has_commits = match self.repo.head() {
            Ok(head) => head.peel_to_commit().is_ok(),
            Err(_) => false,
        };

        if !has_commits {
            println!("📝 '{CCG_BRANCH_NAME}' 分支没有提交，需要创建初始提交...");
            // 返回错误让调用者处理初始提交创建
            return Err(CheckpointError::GitOperationFailed(git2::Error::from_str(
                "Branch has no commits, initial commit needed",
            )));
        } else {
            println!("✅ '{CCG_BRANCH_NAME}' 分支已就绪");
        }

        Ok(())
    }

    /// Get the CCG branch
    pub fn get_ccg_branch(&self) -> CcResult<Branch> {
        self.repo
            .find_branch(CCG_BRANCH_NAME, git2::BranchType::Local)
            .map_err(|e| {
                if e.code() == git2::ErrorCode::NotFound {
                    CheckpointError::BranchNotFound(CCG_BRANCH_NAME.to_string())
                } else {
                    CheckpointError::GitOperationFailed(e)
                }
            })
    }

    /// Switch to the CCG branch
    pub fn switch_to_ccg_branch(&self) -> CcResult<()> {
        let branch = self.get_ccg_branch()?;
        let branch_ref = branch.get();

        self.repo
            .set_head(branch_ref.name().unwrap())
            .map_err(CheckpointError::GitOperationFailed)?;
        Ok(())
    }

    /// Ensure we're on the CCG branch for operations, return original branch name
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
            self.switch_to_ccg_branch()?;
        }

        Ok(current_branch)
    }

    /// Restore to original branch (if not CCG branch)
    pub fn restore_original_branch(&self, original_branch: &str) -> CcResult<()> {
        if original_branch != CCG_BRANCH_NAME {
            let branch_ref = format!("refs/heads/{original_branch}");
            if let Err(e) = self.switch_to_branch(&branch_ref) {
                println!(
                    "{} {} {}",
                    style("⚠️").fg(Color::Yellow),
                    style("警告: 无法切回原始分支").fg(Color::Yellow),
                    style(original_branch).fg(Color::Cyan)
                );
                println!("{} {}", style("错误:").fg(Color::Red), e);
                return Err(e);
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
}
