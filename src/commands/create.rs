use crate::commands::traits::{Command, CommandContext, CreateArgs};
use crate::error::Result as CcResult;
use serde::Deserialize;
use serde_json;
use std::io::{self, Read};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Deserialize, Debug)]
struct StructuredPatch {
    lines: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ToolResponse {
    structured_patch: Option<Vec<StructuredPatch>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
struct HookData {
    tool_name: String,
    tool_response: ToolResponse,
    tool_input: serde_json::Value,
    cwd: Option<String>,
}

/// Create命令实现
pub struct CreateCommand {
    context: CommandContext,
}

impl CreateCommand {
    pub fn new(context: CommandContext) -> Self {
        CreateCommand { context }
    }

    fn format_commit_message(&self, data: &HookData) -> String {
        let file_path = data
            .tool_input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.split('/').next_back().unwrap_or(s))
            .unwrap_or("");

        let title = if file_path.is_empty() {
            data.tool_name.to_string()
        } else {
            format!("{} on {}", data.tool_name, file_path)
        };

        let mut message = format!("{title}\n\n");

        if let Some(patches) = &data.tool_response.structured_patch {
            message.push_str("Changes:\n");
            for patch in patches {
                for line in &patch.lines {
                    message.push_str(&format!("  {line}\n"));
                }
            }
            message.push('\n');
        }

        message.push_str("Tool Input:\n");
        if let Ok(input_pretty) = serde_json::to_string_pretty(&data.tool_input) {
            message.push_str(&input_pretty);
        } else {
            message.push_str(&data.tool_input.to_string());
        }

        message
    }
}

impl Command for CreateCommand {
    type Args = CreateArgs;
    type Output = String;

    fn execute(&self, args: Self::Args) -> CcResult<Self::Output> {
        if let Some(message) = args.message {
            // 如果直接提供了消息，则使用默认上下文
            return self
                .context
                .checkpoint_service
                .create_checkpoint(Some(&message));
        }

        // 尝试从stdin读取
        let mut buffer = String::new();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            if io::stdin().read_to_string(&mut buffer).is_ok() {
                tx.send(buffer).ok();
            }
        });

        if let Ok(stdin_data) = rx.recv_timeout(Duration::from_millis(100)) {
            if !stdin_data.trim().is_empty() {
                return match serde_json::from_str::<HookData>(&stdin_data) {
                    Ok(parsed_data) => {
                        let commit_message = self.format_commit_message(&parsed_data);
                        let context = if let Some(cwd) = parsed_data.cwd {
                            CommandContext::new_with_path(Some(&cwd))?
                        } else {
                            self.context.clone()
                        };
                        context
                            .checkpoint_service
                            .create_checkpoint(Some(&commit_message))
                    }
                    Err(_) => self
                        .context
                        .checkpoint_service
                        .create_checkpoint(Some(&stdin_data)),
                };
            }
        }

        // 如果没有输入，则创建手动检查点
        self.context
            .checkpoint_service
            .create_checkpoint(Some("Manual checkpoint"))
    }

    fn validate_args(&self, _args: &Self::Args) -> CcResult<()> {
        // Create命令的tool_input_json参数是可选的，无需特殊验证
        Ok(())
    }
}
