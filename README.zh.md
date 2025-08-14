# 🛡️ ccg - Claude 代码检查点守护者

[English](README.md)

`ccg` 是一个基于 Git 的检查点管理工具，专为 AI 辅助开发设计。它可以帮助您保存和管理代码的不同阶段，创建一个可以随时查看或恢复的“检查点”。这在与 AI 代码生成器协作时尤其有用，让您可以自由地尝试不同的方案，而不会丢失开发进度。

## ✨ 功能特性

- **✍️ 检查点管理**: 轻松创建、列出和恢复代码检查点。
- **🌳 基于 Git**: 利用 Git 的强大功能和可靠性来管理您的代码历史。
- **🔍 详细差异比较**: 比较不同检查点之间的代码变更，以了解您的开发过程。
- **🌍 国际化**: 支持多种语言（英文和中文）。
- **💻 简洁的命令行界面**: 直观的命令行界面，易于使用。

## 📦 安装

1.  从 [发布页面](https://github.com/your-username/cc-guardian/releases) 下载适用于您系统的二进制文件。
2.  将二进制文件设为可执行，并将其移动到系统 `PATH` 中的一个目录。

### Linux

```bash
# 将二进制文件设为可执行
chmod +x ccg-linux-x86_64

# 将其移动到 PATH 中的一个目录
sudo mv ccg-linux-x86_64 /usr/local/bin/ccg
```

### macOS

```bash
# 将二进制文件设为可执行
chmod +x ccg-macos-x86_64  # 或 ccg-macos-aarch64 (适用于 Apple Silicon)

# 将其移动到 PATH 中的一个目录
sudo mv ccg-macos-x86_64 /usr/local/bin/ccg
```

### Windows

1.  下载 `ccg-windows-x86_64.exe` 文件。
2.  为其创建一个文件夹，例如 `C:\Program Files\ccg`。
3.  将下载的 `.exe` 文件移动到此文件夹中，并将其重命名为 `ccg.exe`。
4.  将该文件夹 (`C:\Program Files\ccg`) 添加到系统的 `Path` 环境变量中。

完成这些步骤后，您应该可以从任何终端运行 `ccg`。

### 🤖 与 Claude Code 集成

为了在 AI 每次修改文件后自动创建检查点，您可以在 Claude Code 的 `settings.json` 文件中配置一个钩子。

将以下 `hooks` 配置添加到您的 `settings.json` 中：

```json
"hooks": {
    "PostToolUse": [
        {
            "matcher": "Edit|MultiEdit|Write",
            "hooks": [
                {
                    "type": "command",
                    "command": "ccg create"
                }
            ]
        }
    ]
}
```

这将确保每当 AI 编辑、多重编辑或写入文件时，都会自动创建一个新的 `ccg` 检查点。

## 🚀 使用方法

### 🎉 初始化 `ccg`

在使用 `ccg` 之前，您需要在项目的仓库中对其进行初始化。

```bash
ccg init
```

### ➕ 创建检查点

将代码的当前状态保存为一个新的检查点。您可以选择性地添加一条消息来描述变更。

```bash
ccg create "实现了新功能"
```

### 📋 列出检查点

查看您创建的所有检查点的列表。

```bash
ccg list
```

您也可以指定要显示的检查点数量：

```bash
ccg list -n 20
```

### ℹ️ 显示检查点详情

查看特定检查点的详细信息，包括其提交信息。

```bash
ccg show <检查点哈希>
```

要查看与检查点相关的代码变更，请使用 `--diff` 或 `-d` 标志：

```bash
ccg show <检查点哈希> --diff
```

### 🔙 恢复检查点

将项目文件恢复到特定检查点的状态。

```bash
ccg restore <检查点哈希>
```

### 👀 比较检查点

查看两个检查点之间的差异。

```bash
ccg diff <哈希A> <哈希B>
```

如果省略第二个哈希，它将与当前工作目录进行比较。

```bash
ccg diff <哈希A>
```

## 💻 本地开发

要设置 `ccg` 进行本地开发：

1.  **克隆仓库：**

    ```bash
    git clone https://github.com/your-username/cc-guardian.git
    cd cc-guardian
    ```

2.  **构建项目：**

    ```bash
    cargo build
    ```

3.  **运行测试：**

    ```bash
    cargo test
    ```

4.  **运行应用程序：**
    ```bash
    cargo run -- <命令>
    ```
    例如，查看帮助信息：
    ```bash
    cargo run -- --help
    ```

## 🤝 贡献

欢迎贡献！请随时提交拉取请求或开启一个 issue。

## 📄 许可证

该项目根据 [MIT 许可证](LICENSE) 授权。
