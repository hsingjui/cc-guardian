use crate::commands::traits::{Command, CommandContext, DiffArgs};
use crate::error::Result as CcResult;

/// Diff命令实现
pub struct DiffCommand {
    context: CommandContext,
}

impl DiffCommand {
    pub fn new(context: CommandContext) -> Self {
        DiffCommand { context }
    }
}

impl Command for DiffCommand {
    type Args = DiffArgs;
    type Output = ();

    fn execute(&self, args: Self::Args) -> CcResult<Self::Output> {
        self.context
            .checkpoint_service
            .diff_checkpoints(&args.hash_a, args.hash_b.as_deref())
    }

    fn validate_args(&self, args: &Self::Args) -> CcResult<()> {
        if args.hash_a.is_empty() {
            return Err(crate::error::CheckpointError::InvalidArgument(
                "第一个检查点哈希值不能为空".to_string(),
            ));
        }
        Ok(())
    }
}
