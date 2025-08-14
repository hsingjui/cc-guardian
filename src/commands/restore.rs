use crate::commands::traits::{Command, CommandContext, RestoreArgs};
use crate::error::Result as CcResult;
use dialoguer::Confirm;

/// Restore命令实现
pub struct RestoreCommand {
    context: CommandContext,
}

impl RestoreCommand {
    pub fn new(context: CommandContext) -> Self {
        RestoreCommand { context }
    }
}

impl Command for RestoreCommand {
    type Args = RestoreArgs;
    type Output = ();

    fn execute(&self, args: Self::Args) -> CcResult<Self::Output> {
        if Confirm::new()
            .with_prompt("您确定要恢复此检查点吗？这将覆盖当前的工作目录。")
            .interact()?
        {
            println!("正在恢复检查点...");
            self.context
                .checkpoint_service
                .restore_checkpoint(&args.hash)?;
            println!("检查点 {} 已成功恢复。", args.hash);
        } else {
            println!("恢复操作已取消。");
        }
        Ok(())
    }

    fn validate_args(&self, args: &Self::Args) -> CcResult<()> {
        if args.hash.is_empty() {
            return Err(crate::error::CheckpointError::InvalidArgument(
                "检查点哈希值不能为空".to_string(),
            ));
        }
        Ok(())
    }
}
