//! Repository initialization and basic operations
//!
//! This module handles repository-level operations including initialization,
//! opening existing repositories, and basic validation.

use super::types::CCG_BRANCH_NAME;
use crate::error::{CheckpointError, Result as CcResult};
use git2::Repository;
use std::path::Path;

/// Operations related to repository initialization and management
///
/// This struct provides methods for creating, opening, and validating
/// Git repositories. It holds a reference to the underlying libgit2 Repository.
pub struct RepositoryOperations<'a> {
    repo: &'a Repository,
}

impl<'a> RepositoryOperations<'a> {
    /// Create a new RepositoryOperations instance
    ///
    /// # Arguments
    /// * `repo` - Reference to the Git repository
    pub fn new(repo: &'a Repository) -> Self {
        Self { repo }
    }

    /// Initialize a new Git repository in the current directory
    pub fn init_repository(path: &str) -> CcResult<Repository> {
        println!("📁 在 '{path}' 目录检测到不是Git仓库，正在初始化...");

        // 初始化Git仓库
        let repo = Repository::init(path).map_err(CheckpointError::GitOperationFailed)?;

        // 立即设置HEAD指向ccg分支（即使分支还不存在）
        // 这样第一个提交就会创建ccg分支而不是master/main分支
        let ccg_ref = format!("refs/heads/{CCG_BRANCH_NAME}");
        repo.set_head(&ccg_ref)
            .map_err(CheckpointError::GitOperationFailed)?;

        println!("✅ Git仓库初始化成功");

        Ok(repo)
    }

    /// Open an existing Git repository from a path
    ///
    /// # Arguments
    /// * `path` - Path to the repository directory
    ///
    /// # Returns
    /// * `Ok(Repository)` - Successfully opened repository
    /// * `Err(CheckpointError)` - Repository not found or other error
    pub fn open_repository<P: AsRef<Path>>(path: P) -> CcResult<Repository> {
        Repository::open(path).map_err(|e| match e.class() {
            git2::ErrorClass::Repository => CheckpointError::RepositoryNotFound,
            _ => CheckpointError::GitOperationFailed(e),
        })
    }

    /// Validate that the repository is in a good state
    pub fn validate_repository(&self) -> CcResult<()> {
        // Check if we can access the repository path
        let _path = self.repo.path();

        // Try to access the repository's configuration
        let _config = self
            .repo
            .config()
            .map_err(CheckpointError::GitOperationFailed)?;

        Ok(())
    }

    /// Get the path to the repository's .git directory
    ///
    /// # Returns
    /// Path to the repository's .git directory
    pub fn get_repository_path(&self) -> &Path {
        self.repo.path()
    }

    /// Get the path to the repository's working directory
    ///
    /// # Returns
    /// * `Some(Path)` - Path to working directory if it exists
    /// * `None` - If this is a bare repository
    pub fn get_workdir_path(&self) -> Option<&Path> {
        self.repo.workdir()
    }
}
