use crate::error::{CheckpointError, Result as CcResult};
use crate::git_ops::GitOperations;
use console::{Color, style};

/// 检查点服务，封装检查点相关的业务逻辑
#[derive(Clone)]
pub struct CheckpointService {
    git_ops: GitOperations,
}

impl CheckpointService {
    pub fn new(git_ops: GitOperations) -> CcResult<Self> {
        Ok(CheckpointService { git_ops })
    }

    /// 在ccg分支上执行操作的通用包装器
    fn execute_on_ccg_branch<F, R>(&self, operation: F) -> CcResult<R>
    where
        F: FnOnce(&GitOperations) -> CcResult<R>,
    {
        // 确保在ccg分支上执行
        let original_branch = match self.git_ops.ensure_ccg_branch() {
            Ok(branch) => branch,
            Err(CheckpointError::BranchNotFound(_)) => {
                // 如果ccg分支不存在，则初始化它
                println!(
                    "{} {}",
                    style("ℹ️").fg(Color::Blue),
                    style("未找到 'ccg' 分支，将自动初始化...").fg(Color::White)
                );
                self.git_ops.init_checkpoints()?;
                // 初始化后，再次确保切换到ccg分支
                self.git_ops.ensure_ccg_branch()?
            }
            Err(CheckpointError::GitOperationFailed(e))
                if e.code() == git2::ErrorCode::NotFound
                    || e.code() == git2::ErrorCode::UnbornBranch =>
            {
                println!(
                    "{} {}",
                    style("ℹ️").fg(Color::Blue),
                    style("未找到 'ccg' 分支或仓库未初始化，将自动初始化...").fg(Color::White)
                );
                self.git_ops.init_checkpoints()?;
                self.git_ops.ensure_ccg_branch()?
            }
            Err(e) => return Err(e),
        };

        // 执行操作
        let result = operation(&self.git_ops);

        // 恢复原始分支
        if let Err(_restore_err) = self.git_ops.restore_original_branch(&original_branch) {
            // 如果恢复分支失败，但操作成功，我们仍然返回操作结果，但记录警告
            if result.is_ok() {
                println!(
                    "{} {}",
                    style("⚠️").fg(Color::Yellow),
                    style("操作成功完成，但分支恢复失败").fg(Color::Yellow)
                );
            }
        }

        result
    }

    /// 初始化检查点系统
    pub fn init(&self) -> CcResult<()> {
        println!(
            "{} {}",
            style("🚀").fg(Color::Blue),
            style("初始化 Claude Code Checkpoint Guardian")
                .fg(Color::Cyan)
                .bold()
        );

        // 初始化检查点系统（会自动处理Git仓库和ccg分支）
        self.git_ops.init_checkpoints()?;

        // 检查是否是新初始化的Git仓库
        let current_branch = self.git_ops.get_current_branch_name()?;
        if current_branch == "ccg" {
            println!(
                "{} {}",
                style("✅").fg(Color::Green),
                style("Claude Code Checkpoint Guardian 初始化完成！")
                    .fg(Color::Green)
                    .bold()
            );
            println!(
                "{} {} {}",
                style("📍").fg(Color::Blue),
                style("当前分支:").fg(Color::White),
                style(&current_branch).fg(Color::Yellow).bold()
            );
            println!(
                "{} {}",
                style("💡").fg(Color::Yellow),
                style("提示: 现在可以使用 'ccg create' 创建检查点").fg(Color::White)
            );
        } else {
            println!(
                "{} {} {}",
                style("⚠️").fg(Color::Yellow),
                style("当前分支:").fg(Color::White),
                style(&current_branch).fg(Color::Yellow).bold()
            );
            println!(
                "{} {}",
                style("💡").fg(Color::Yellow),
                style("提示: ccg 分支已准备就绪，使用 'git checkout ccg' 切换").fg(Color::White)
            );
        }

        Ok(())
    }

    /// 创建检查点
    pub fn create_checkpoint(&self, tool_input: Option<&str>) -> CcResult<String> {
        println!(
            "{} {}",
            style("🔄").fg(Color::Blue),
            style("开始创建检查点...").fg(Color::White)
        );

        self.execute_on_ccg_branch(|git_ops| {
            let message = tool_input.unwrap_or("Checkpoint created without a specific message.");

            match git_ops.create_checkpoint(message) {
                Ok(hash) => {
                    let short_hash = &hash[..7];
                    println!(
                        "{} {}",
                        style("✅ Created checkpoint:").fg(Color::Green).bold(),
                        style(short_hash).fg(Color::Yellow).bold(),
                    );
                    Ok(hash)
                }
                Err(CheckpointError::NoChangesToCommit) => {
                    println!(
                        "{} {}",
                        style("ℹ️").fg(Color::Blue),
                        style("没有检测到文件变更，跳过创建检查点").fg(Color::Yellow)
                    );
                    Ok(String::new())
                }
                Err(e) => Err(e),
            }
        })
    }

    /// 列出检查点
    pub fn list_checkpoints(&self, number: usize) -> CcResult<()> {
        self.execute_on_ccg_branch(|git_ops| {
            let checkpoints = git_ops.list_checkpoints(number)?;
            if checkpoints.is_empty() {
                println!("{}", style("📭 No checkpoints found.").fg(Color::Yellow));
            } else {
                println!(
                    "{}",
                    style("📋 Recent checkpoints:").fg(Color::Green).bold()
                );
                println!();
                for (i, checkpoint) in checkpoints.iter().enumerate() {
                    let prefix = if i == 0 {
                        style("  ●").fg(Color::Green).bold()
                    } else {
                        style("  ○").fg(Color::Blue)
                    };
                    println!("{prefix} {checkpoint}");
                }
            }
            Ok(())
        })
    }

    /// 恢复检查点 - 真正的时光机效果，丢弃后续提交
    pub fn restore_checkpoint(&self, hash: &str) -> CcResult<()> {
        let short_hash = if hash.len() >= 7 { &hash[..7] } else { hash };

        // 记录当前分支
        let original_branch = self.git_ops.get_current_branch_name()?;

        // 确保在 ccg 分支上执行
        self.git_ops.ensure_ccg_branch()?;

        // 安全检查：检查是否有未提交的更改
        if self.git_ops.has_uncommitted_changes()? {
            // 如果有未提交更改，恢复到原始分支
            if original_branch != "ccg" {
                let _ = self.git_ops.restore_original_branch(&original_branch);
            }

            println!(
                "{} {}",
                style("⚠️").fg(Color::Yellow),
                style("检测到未提交的更改。恢复检查点将会丢失这些更改。").fg(Color::Yellow)
            );
            println!(
                "{} {}",
                style("💡").fg(Color::Blue),
                style("建议先提交或暂存您的更改，然后再恢复检查点。").fg(Color::White)
            );
            return Err(CheckpointError::UncommittedChanges);
        }

        // 获取目标检查点信息，用于确认操作
        let target_commit = self.git_ops.find_commit(hash)?;
        let current_head = self.git_ops.get_head_commit()?;

        // 检查是否会丢失后续提交
        let commits_ahead = self.git_ops.count_commits_between(
            &target_commit.id().to_string(),
            &current_head.id().to_string(),
        )?;

        if commits_ahead > 0 {
            println!(
                "{} {} {} {}",
                style("⚠️").fg(Color::Yellow),
                style("警告: 此操作将丢失").fg(Color::Yellow),
                style(commits_ahead.to_string()).fg(Color::Red).bold(),
                style("个后续检查点").fg(Color::Yellow)
            );

            // 可以在这里添加确认提示，但现在直接执行
            println!(
                "{} {}",
                style("�").fg(Color::Red),
                style("继续执行将永久丢失这些检查点!").fg(Color::Red).bold()
            );
        }

        println!(
            "{} {} {}",
            style("�").fg(Color::Blue),
            style("恢复到检查点并重置分支:").fg(Color::White),
            style(short_hash).fg(Color::Yellow).bold()
        );

        // 执行硬重置操作 - 这是关键变化
        self.git_ops.reset_branch_to_checkpoint(hash)?;

        println!(
            "{} {} {}",
            style("✅").fg(Color::Green),
            style("成功恢复到检查点:").fg(Color::Green).bold(),
            style(short_hash).fg(Color::Yellow).bold()
        );

        // 显示当前状态信息
        println!(
            "{} {}",
            style("📍").fg(Color::Blue),
            style("ccg 分支已重置到指定检查点，后续提交已被丢弃").fg(Color::White)
        );

        // 如果原始分支不是 ccg，提供切换提示
        if original_branch != "ccg" {
            println!(
                "{} {}",
                style("💡").fg(Color::Yellow),
                style("提示: 你现在在 ccg 分支上").fg(Color::White)
            );
            println!(
                "  {} {} {}",
                style("•").fg(Color::Blue),
                style("使用 'git switch").fg(Color::White),
                style(&format!("{original_branch}' 返回原始分支")).fg(Color::Cyan)
            );
        }

        Ok(())
    }

    /// 显示检查点详情
    pub fn show_checkpoint(&self, hash: &str, show_diff: bool) -> CcResult<()> {
        self.execute_on_ccg_branch(|git_ops| {
            // 先查找提交以获取完整hash和短hash显示
            match git_ops.find_commit(hash) {
                Ok(commit) => {
                    let full_hash = commit.id().to_string();
                    let short_hash = &full_hash[..7];

                    println!(
                        "{} {} {}",
                        style("📋").fg(Color::Blue),
                        style("Checkpoint details for").fg(Color::White),
                        style(short_hash).fg(Color::Yellow).bold()
                    );
                    println!();

                    let details = git_ops.show_checkpoint(hash, show_diff)?;
                    println!("{details}");
                    Ok(())
                }
                Err(CheckpointError::InvalidHash(msg)) => {
                    // 如果是多个匹配的错误，直接显示错误信息
                    println!(
                        "{} {}",
                        style("❌").fg(Color::Red),
                        style(&msg).fg(Color::Yellow)
                    );
                    Ok(())
                }
                Err(e) => Err(e),
            }
        })
    }

    /// 比较检查点差异
    pub fn diff_checkpoints(&self, hash_a: &str, hash_b: Option<&str>) -> CcResult<()> {
        self.execute_on_ccg_branch(|git_ops| {
            let short_hash_a = if hash_a.len() >= 7 {
                &hash_a[..7]
            } else {
                hash_a
            };
            let diff = git_ops.diff_checkpoints(hash_a, hash_b)?;

            if let Some(hash_b) = hash_b {
                let short_hash_b = if hash_b.len() >= 7 {
                    &hash_b[..7]
                } else {
                    hash_b
                };
                println!(
                    "{} {} {} {} {}",
                    style("🔍").fg(Color::Blue),
                    style("Differences between").fg(Color::White),
                    style(short_hash_a).fg(Color::Yellow).bold(),
                    style("and").fg(Color::White),
                    style(short_hash_b).fg(Color::Yellow).bold()
                );
            } else {
                println!(
                    "{} {} {} {} {}",
                    style("🔍").fg(Color::Blue),
                    style("Differences between").fg(Color::White),
                    style(short_hash_a).fg(Color::Yellow).bold(),
                    style("and").fg(Color::White),
                    style("working directory").fg(Color::Cyan)
                );
            }
            println!();
            println!("{diff}");
            Ok(())
        })
    }

    /// 清理旧检查点
    pub fn prune_checkpoints(&self, keep: Option<usize>, before: Option<&str>) -> CcResult<()> {
        self.execute_on_ccg_branch(|git_ops| {
            git_ops.prune_checkpoints(keep, before)?;
            println!(
                "{} {}",
                style("🗑️").fg(Color::Red),
                style("Pruned old checkpoints.").fg(Color::Green).bold()
            );
            Ok(())
        })
    }
}
