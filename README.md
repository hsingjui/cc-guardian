# 🛡️ ccg - Claude Code Checkpoint Guardian

[简体中文](README.zh.md)

`ccg` is a Git-based checkpoint management tool designed for AI-assisted development. It helps you save and manage different stages of your code, creating a "checkpoint" that you can inspect or revert to at any time. This is particularly useful when working with AI code generators, allowing you to experiment freely with different approaches without losing track of your progress.

## ✨ Features

- **✍️ Checkpoint Management**: Easily create, list, and restore code checkpoints.
- **🌳 Git-based**: Leverages the power and reliability of Git to manage your code's history.
- **🔍 Detailed Diffs**: Compare the changes between different checkpoints to understand your development process.
- **🌍 Internationalization**: Supports multiple languages (English and Chinese).
- **💻 Simple CLI**: An intuitive command-line interface for easy use.

## 📦 Installation

1.  Download the appropriate binary for your system from the [Releases](https://github.com/your-username/cc-guardian/releases) page.
2.  Make the binary executable and move it to a directory in your system's `PATH`.

### Linux

```bash
# Make the binary executable
chmod +x ccg-linux-x86_64

# Move it to a directory in your PATH
sudo mv ccg-linux-x86_64 /usr/local/bin/ccg
```

### macOS

```bash
# Make the binary executable
chmod +x ccg-macos-x86_64  # Or ccg-macos-aarch64 for Apple Silicon

# Move it to a directory in your PATH
sudo mv ccg-macos-x86_64 /usr/local/bin/ccg
```

### Windows

1.  Download the `ccg-windows-x86_64.exe` file.
2.  Create a folder for it, for example, `C:\Program Files\ccg`.
3.  Move the downloaded `.exe` file into this folder and rename it to `ccg.exe`.
4.  Add the folder (`C:\Program Files\ccg`) to your system's `Path` environment variable.

After these steps, you should be able to run `ccg` from any terminal.

### 🤖 Integration with Claude Code

To automatically create a checkpoint after every file modification made by the AI, you can configure a hook in your Claude Code `settings.json` file.

Add the following `hooks` configuration to your `settings.json`:

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

This will ensure that every time the AI edits, multi-edits, or writes a file, a new `ccg` checkpoint is automatically created.

## 🚀 Usage

### Initialize `ccg`

Before using `ccg`, you need to initialize it in your project's repository.

```bash
ccg init
```

### ➕ Create a Checkpoint

Save the current state of your code as a new checkpoint. You can optionally add a message to describe the changes.

```bash
ccg create "Implemented the new feature"
```

### 📋 List Checkpoints

View a list of all the checkpoints you've created.

```bash
ccg list
```

You can also specify the number of checkpoints to show:

```bash
ccg list -n 20
```

### ℹ️ Show Checkpoint Details

View the details of a specific checkpoint, including its commit information.

```bash
ccg show <checkpoint_hash>
```

To see the code changes associated with a checkpoint, use the `--diff` or `-d` flag:

```bash
ccg show <checkpoint_hash> --diff
```

### 🔙 Restore a Checkpoint

Revert your project's files to the state of a specific checkpoint.

```bash
ccg restore <checkpoint_hash>
```

### 👀 Compare Checkpoints

See the difference between two checkpoints.

```bash
ccg diff <hash_a> <hash_b>
```

If you omit the second hash, it will be compared against the current working directory.

```bash
ccg diff <hash_a>
```

## 💻 Local Development

To set up `ccg` for local development:

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/your-username/cc-guardian.git
    cd cc-guardian
    ```

2.  **Build the project:**

    ```bash
    cargo build
    ```

3.  **Run the tests:**

    ```bash
    cargo test
    ```

4.  **Run the application:**
    ```bash
    cargo run -- <command>
    ```
    For example, to see the help message:
    ```bash
    cargo run -- --help
    ```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

## 📄 License

This project is licensed under the [MIT License](LICENSE).
