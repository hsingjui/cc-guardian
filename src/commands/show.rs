use crate::commands::traits::{Command, CommandContext, ShowArgs};
use crate::error::Result as CcResult;

/// Show命令实现
pub struct ShowCommand {
    context: CommandContext,
}

impl ShowCommand {
    pub fn new(context: CommandContext) -> Self {
        ShowCommand { context }
    }
}

impl Command for ShowCommand {
    type Args = ShowArgs;
    type Output = ();

    fn execute(&self, args: Self::Args) -> CcResult<Self::Output> {
        self.context
            .checkpoint_service
            .show_checkpoint(&args.hash, args.diff)
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
