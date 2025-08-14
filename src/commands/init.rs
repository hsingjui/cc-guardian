use crate::commands::traits::{Command, CommandContext, InitArgs};
use crate::error::Result as CcResult;

/// Init命令实现
pub struct InitCommand {
    context: CommandContext,
}

impl InitCommand {
    pub fn new(context: CommandContext) -> Self {
        InitCommand { context }
    }
}

impl Command for InitCommand {
    type Args = InitArgs;
    type Output = ();

    fn execute(&self, _args: Self::Args) -> CcResult<Self::Output> {
        self.context.checkpoint_service.init()
    }

    fn validate_args(&self, _args: &Self::Args) -> CcResult<()> {
        // Init命令无参数需要验证
        Ok(())
    }
}
