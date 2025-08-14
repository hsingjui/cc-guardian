use crate::error::Result as CcResult;
use crate::git_ops::GitOperations;
use crate::services::CheckpointService;

/// 统一的命令接口
pub trait Command {
    type Args;
    type Output;

    /// 执行命令
    fn execute(&self, args: Self::Args) -> CcResult<Self::Output>;

    /// 验证命令参数
    fn validate_args(&self, args: &Self::Args) -> CcResult<()> {
        // 默认实现：无验证
        let _ = args;
        Ok(())
    }
}

/// 命令执行上下文，提供共享资源
#[derive(Clone)]
pub struct CommandContext {
    pub git_ops: GitOperations,
    pub checkpoint_service: CheckpointService,
}

impl CommandContext {
    pub fn new() -> CcResult<Self> {
        Self::new_with_path(None)
    }

    pub fn new_with_path(path: Option<&str>) -> CcResult<Self> {
        let git_ops = GitOperations::new(path)?;
        let checkpoint_service = CheckpointService::new(git_ops.clone())?;

        Ok(CommandContext {
            git_ops,
            checkpoint_service,
        })
    }
}

// 命令参数结构体定义

/// Init命令参数（无参数）
#[derive(Debug, Clone)]
pub struct InitArgs;

/// Create命令参数
#[derive(Debug, Clone)]
pub struct CreateArgs {
    pub message: Option<String>,
}

/// List命令参数
#[derive(Debug, Clone)]
pub struct ListArgs {
    pub number: usize,
}

/// Restore命令参数
#[derive(Debug, Clone)]
pub struct RestoreArgs {
    pub hash: String,
}

/// Show命令参数
#[derive(Debug, Clone)]
pub struct ShowArgs {
    pub hash: String,
    pub diff: bool,
}

/// Diff命令参数
#[derive(Debug, Clone)]
pub struct DiffArgs {
    pub hash_a: String,
    pub hash_b: Option<String>,
}

/// Prune命令参数
#[derive(Debug, Clone)]
pub struct PruneArgs {
    pub keep: Option<usize>,
    pub before: Option<String>,
}
