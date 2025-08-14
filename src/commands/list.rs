use crate::commands::traits::{Command, CommandContext, ListArgs};
use crate::error::Result as CcResult;

/// List命令实现
pub struct ListCommand {
    context: CommandContext,
}

impl ListCommand {
    pub fn new(context: CommandContext) -> Self {
        ListCommand { context }
    }
}

impl Command for ListCommand {
    type Args = ListArgs;
    type Output = ();

    fn execute(&self, args: Self::Args) -> CcResult<Self::Output> {
        self.context
            .checkpoint_service
            .list_checkpoints(args.number)
    }

    fn validate_args(&self, args: &Self::Args) -> CcResult<()> {
        if args.number == 0 {
            return Err(crate::error::CheckpointError::InvalidArgument(
                "显示数量必须大于0".to_string(),
            ));
        }
        Ok(())
    }
}
