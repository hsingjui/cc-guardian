//! Diff operations module
//!
//! This module handles all diff-related operations including generating diffs,
//! formatting diff output, and calculating diff statistics.

use crate::error::{CheckpointError, Result as CcResult};
use crate::git_ops::types::{DiffStats, FileChangeInfo};
use console::{style, Color};
use git2::{Commit, Diff, Repository};
use std::collections::HashMap;

/// Operations for handling git diffs and comparisons
///
/// This struct provides methods for generating, formatting, and analyzing
/// differences between commits, trees, and the working directory.
pub struct DiffOperations<'a> {
    /// Reference to the git repository
    repo: &'a Repository,
}

impl<'a> DiffOperations<'a> {
    /// Create a new DiffOperations instance
    ///
    /// # Arguments
    /// * `repo` - Reference to the git repository
    ///
    /// # Returns
    /// A new DiffOperations instance
    pub fn new(repo: &'a Repository) -> Self {
        Self { repo }
    }

    /// Get the diff for a specific commit
    ///
    /// Generates a diff showing the changes introduced by the given commit.
    /// For the first commit, compares against an empty tree.
    ///
    /// # Arguments
    /// * `commit` - The commit to generate a diff for
    ///
    /// # Returns
    /// A git2::Diff object representing the changes
    ///
    /// # Errors
    /// Returns CheckpointError::GitOperationFailed if the diff cannot be generated
    pub fn get_commit_diff(&self, commit: &Commit) -> CcResult<Diff> {
        if let Ok(parent) = commit.parent(0) {
            let tree_a = parent.tree()?;
            let tree_b = commit.tree()?;
            self.repo
                .diff_tree_to_tree(Some(&tree_a), Some(&tree_b), None)
                .map_err(CheckpointError::GitOperationFailed)
        } else {
            // This is the first commit, compare against empty tree
            let tree_b = commit.tree()?;
            self.repo
                .diff_tree_to_tree(None, Some(&tree_b), None)
                .map_err(CheckpointError::GitOperationFailed)
        }
    }

    /// Compare two commits and generate a formatted diff
    ///
    /// Generates a human-readable diff between two commits or between
    /// a commit and the working directory.
    ///
    /// # Arguments
    /// * `hash_a` - Hash of the first commit
    /// * `hash_b` - Optional hash of the second commit. If None, compares with working directory
    ///
    /// # Returns
    /// A formatted string containing the diff output
    ///
    /// # Errors
    /// Returns CheckpointError if commits cannot be found or diff cannot be generated
    pub fn diff_commits(&self, hash_a: &str, hash_b: Option<&str>) -> CcResult<String> {
        // Find the first commit using the commit operations logic
        let commit_a = self.find_commit_by_hash(hash_a)?;
        let tree_a = commit_a.tree()?;

        let diff = if let Some(hash_b) = hash_b {
            let commit_b = self.find_commit_by_hash(hash_b)?;
            let tree_b = commit_b.tree()?;
            self.repo
                .diff_tree_to_tree(Some(&tree_a), Some(&tree_b), None)?
        } else {
            // Compare with working directory
            self.repo.diff_tree_to_index(Some(&tree_a), None, None)?
        };

        self.format_diff_output(&diff)
    }

    /// Get diff between working directory and HEAD
    ///
    /// Generates a diff showing uncommitted changes in the working directory.
    ///
    /// # Returns
    /// A git2::Diff object representing the working directory changes
    ///
    /// # Errors
    /// Returns CheckpointError::GitOperationFailed if the diff cannot be generated
    pub fn get_workdir_diff(&self) -> CcResult<Diff> {
        let head = self.repo.head()?;
        let head_commit = head.peel_to_commit()?;
        let head_tree = head_commit.tree()?;

        self.repo
            .diff_tree_to_index(Some(&head_tree), None, None)
            .map_err(CheckpointError::GitOperationFailed)
    }

    /// Get formatted diff content for a specific commit
    ///
    /// This is the main method for generating detailed, colored diff output
    /// with intelligent formatting and line number display.
    ///
    /// # Arguments
    /// * `hash` - The commit hash to generate diff content for
    ///
    /// # Returns
    /// A formatted string with colored diff output including statistics
    ///
    /// # Errors
    /// Returns CheckpointError if the commit cannot be found or diff cannot be generated
    pub fn get_commit_diff_content(&self, hash: &str) -> CcResult<String> {
        let commit = self.find_commit_by_hash(hash)?;
        let diff = self.get_commit_diff(&commit)?;
        self.format_diff_output(&diff)
    }

    /// Calculate statistics for a diff
    ///
    /// Analyzes a git2::Diff object and returns aggregated statistics
    /// including file counts and line change information.
    ///
    /// # Arguments
    /// * `diff` - The git2::Diff object to analyze
    ///
    /// # Returns
    /// A DiffStats struct containing the calculated statistics
    pub fn calculate_diff_stats(&self, diff: &Diff) -> DiffStats {
        let mut stats = DiffStats::new();
        let mut file_changes = Vec::new();

        // Collect file-level statistics
        for delta in diff.deltas() {
            if let Some(new_file) = delta.new_file().path() {
                let file_path = new_file.to_string_lossy().to_string();
                let file_change = FileChangeInfo::new(file_path, delta.status());
                file_changes.push(file_change);

                // Count file modifications by type
                if delta.status() == git2::Delta::Modified {
                    stats.modifications += 1;
                }
            }
        }

        stats.total_files = file_changes.len();

        // Calculate line-level statistics by processing the diff
        let _ = diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let origin = line.origin();

            // Skip special markers and binary content indicators
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            if content.contains("No newline at end of file")
                || content.contains("\\ No newline at end of file")
                || origin == '>'
                || origin == '<'
            {
                return true;
            }

            match origin {
                '+' => {
                    stats.additions += 1;
                }
                '-' => {
                    stats.deletions += 1;
                }
                _ => {} // Context lines and headers don't count
            }
            true
        });

        stats
    }

    /// Get a summary string of diff statistics
    ///
    /// Creates a human-readable summary of the changes in a diff.
    ///
    /// # Arguments
    /// * `diff` - The git2::Diff object to summarize
    ///
    /// # Returns
    /// A formatted string summarizing the diff statistics
    pub fn get_diff_summary(&self, diff: &Diff) -> String {
        let stats = self.calculate_diff_stats(diff);

        let mut summary = format!("{} files changed", stats.total_files);

        if stats.additions > 0 || stats.deletions > 0 {
            summary.push_str(", ");

            let mut parts = Vec::new();
            if stats.additions > 0 {
                parts.push(format!("{} insertions(+)", stats.additions));
            }
            if stats.deletions > 0 {
                parts.push(format!("{} deletions(-)", stats.deletions));
            }

            summary.push_str(&parts.join(", "));
        }

        summary
    }

    /// Helper method to find a commit by hash (supports short hashes)
    ///
    /// This is a simplified version of the commit finding logic.
    /// In the full implementation, this would delegate to CommitOperations.
    ///
    /// # Arguments
    /// * `hash` - Full or partial commit hash
    ///
    /// # Returns
    /// The found commit
    ///
    /// # Errors
    /// Returns CheckpointError if the commit cannot be found
    fn find_commit_by_hash(&self, hash: &str) -> CcResult<Commit> {
        // First try complete hash
        if let Ok(oid) = git2::Oid::from_str(hash) {
            if let Ok(commit) = self.repo.find_commit(oid) {
                return Ok(commit);
            }
        }

        // If complete hash fails, try short hash query
        if hash.len() >= 2 && hash.len() < 40 {
            let mut revwalk = self
                .repo
                .revwalk()
                .map_err(CheckpointError::GitOperationFailed)?;
            revwalk
                .set_sorting(git2::Sort::TIME)
                .map_err(CheckpointError::GitOperationFailed)?;
            revwalk
                .push_head()
                .map_err(CheckpointError::GitOperationFailed)?;

            let mut matches = Vec::new();
            for oid_result in revwalk {
                let oid = oid_result.map_err(CheckpointError::GitOperationFailed)?;
                let oid_str = oid.to_string();

                if oid_str.starts_with(hash) {
                    matches.push(oid);
                }
            }

            match matches.len() {
                0 => Err(CheckpointError::CheckpointNotFound(hash.to_string())),
                1 => {
                    let commit = self
                        .repo
                        .find_commit(matches[0])
                        .map_err(CheckpointError::GitOperationFailed)?;
                    Ok(commit)
                }
                _ => {
                    let error_msg = format!("短hash '{hash}' 匹配到多个提交，请使用更长的hash前缀");
                    Err(CheckpointError::InvalidHash(error_msg))
                }
            }
        } else {
            Err(CheckpointError::InvalidHash(format!(
                "无效的hash格式: {hash}"
            )))
        }
    }

    /// Format a git2::Diff object into a human-readable string
    ///
    /// This method handles the complex formatting logic including:
    /// - File status indicators with colors
    /// - Line number display
    /// - Intelligent newline handling
    /// - Statistics summary
    ///
    /// # Arguments
    /// * `diff` - The git2::Diff object to format
    ///
    /// # Returns
    /// A formatted string with colored diff output
    ///
    /// # Errors
    /// Returns CheckpointError::GitOperationFailed if formatting fails
    pub fn format_diff_output(&self, diff: &Diff) -> CcResult<String> {
        let mut result = String::new();
        let mut current_file = String::new();
        let mut file_stats = HashMap::new();
        let mut old_line_num = 1;
        let mut new_line_num = 1;
        let mut hunk_initialized = false;

        // First collect file statistics
        for delta in diff.deltas() {
            if let Some(new_file) = delta.new_file().path() {
                let file_path = new_file.to_string_lossy().to_string();
                file_stats.insert(file_path, (0, 0)); // (additions, deletions)
            }
        }

        // First collect file statistics
        for delta in diff.deltas() {
            if let Some(new_file) = delta.new_file().path() {
                let file_path = new_file.to_string_lossy().to_string();
                file_stats.insert(file_path, (0, 0)); // (additions, deletions)
            }
        }

        // Variables for intelligent newline handling
        let mut pending_deletions: Vec<(String, i32)> = Vec::new();
        let mut pending_additions: Vec<(String, i32)> = Vec::new();
        let mut in_newline_context = false;

        // Generate formatted diff output
        diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
            let origin = line.origin();
            let content = std::str::from_utf8(line.content()).unwrap_or("<binary>");

            // Detect newline-related special cases
            if content.contains("No newline at end of file")
                || content.contains("\\ No newline at end of file")
                || origin == '>'
                || origin == '<'
            {
                in_newline_context = true;
                return true; // Skip these marker lines
            }

            // In newline context, collect + and - changes
            if in_newline_context && (origin == '+' || origin == '-') {
                if origin == '+' {
                    pending_additions.push((content.to_string(), new_line_num));
                } else if origin == '-' {
                    pending_deletions.push((content.to_string(), old_line_num));
                }
                return true;
            }

            match origin {
                'F' => {
                    // File header information
                    if content.starts_with("diff --git") {
                        if !current_file.is_empty() {
                            result.push('\n');
                        }

                        // Extract filename
                        if let Some(new_file) = delta.new_file().path() {
                            current_file = new_file.to_string_lossy().to_string();
                            // Reset line numbers and hunk initialization flag for new file
                            hunk_initialized = false;
                            old_line_num = 0;
                            new_line_num = 0;

                            // Add file separator
                            result.push_str(&format!(
                                "{}\n",
                                style("─".repeat(100)).fg(Color::Blue).dim()
                            ));

                            // File status indicator
                            let (status_icon, status_text, status_color) = match delta.status() {
                                git2::Delta::Added => ("📄", "新增文件", Color::Green),
                                git2::Delta::Deleted => ("🗑️", "删除文件", Color::Red),
                                git2::Delta::Modified => ("📝", "修改文件", Color::Yellow),
                                git2::Delta::Renamed => ("📋", "重命名文件", Color::Blue),
                                git2::Delta::Copied => ("📑", "复制文件", Color::Magenta),
                                _ => ("📄", "文件变更", Color::White),
                            };

                            result.push_str(&format!(
                                "{} {} {}\n",
                                style(status_icon).fg(status_color),
                                style(status_text).fg(status_color).bold(),
                                style(&current_file).fg(Color::Cyan).bold()
                            ));
                        }
                    } else if content.starts_with("index ") {
                        // Show file mode information (if changed)
                        result.push_str(&format!(
                            "{} {}\n",
                            style("📋").fg(Color::Blue),
                            style(content.trim()).fg(Color::Blue).dim()
                        ));
                    }
                }
                'H' => {
                    // New hunk starts, first process previous pending changes
                    if in_newline_context
                        && (!pending_deletions.is_empty() || !pending_additions.is_empty())
                    {
                        // Intelligently handle newline-related changes
                        self.handle_pending_newline_changes(
                            &mut result,
                            &pending_deletions,
                            &pending_additions,
                            &mut file_stats,
                            &current_file,
                        );

                        // Clear pending changes
                        pending_deletions.clear();
                        pending_additions.clear();
                        in_newline_context = false;
                    }

                    // Hunk header information - unified parsing of line number ranges
                    // Prefer git2 provided hunk information, otherwise parse manually
                    if let Some(hunk) = hunk {
                        old_line_num = hunk.old_start() as i32;
                        new_line_num = hunk.new_start() as i32;
                        hunk_initialized = true;

                        result.push_str(&format!(
                            "{} {} {} {} {}\n",
                            style("📍").fg(Color::Cyan),
                            style("行号范围:").fg(Color::Cyan).bold(),
                            style(format!(
                                "旧文件:{}-{}",
                                hunk.old_start(),
                                if hunk.old_lines() > 0 {
                                    hunk.old_start() + hunk.old_lines() - 1
                                } else {
                                    hunk.old_start()
                                }
                            ))
                            .fg(Color::Red)
                            .bold(),
                            style("→").fg(Color::White),
                            style(format!(
                                "新文件:{}-{}",
                                hunk.new_start(),
                                if hunk.new_lines() > 0 {
                                    hunk.new_start() + hunk.new_lines() - 1
                                } else {
                                    hunk.new_start()
                                }
                            ))
                            .fg(Color::Green)
                            .bold()
                        ));
                    } else if content.starts_with("@@") {
                        // Manually parse hunk header information
                        let parts: Vec<&str> = content.split_whitespace().collect();
                        if parts.len() >= 3 {
                            // Parse -old_start,old_count
                            if let Some(old_part) = parts.get(1) {
                                if let Some(old_start_str) = old_part.strip_prefix('-') {
                                    if let Some(comma_pos) = old_start_str.find(',') {
                                        if let Ok(start) = old_start_str[..comma_pos].parse::<i32>()
                                        {
                                            old_line_num = start;
                                            hunk_initialized = true;
                                        }
                                    } else if let Ok(start) = old_start_str.parse::<i32>() {
                                        old_line_num = start;
                                        hunk_initialized = true;
                                    }
                                }
                            }
                            // Parse +new_start,new_count
                            if let Some(new_part) = parts.get(2) {
                                if let Some(new_start_str) = new_part.strip_prefix('+') {
                                    if let Some(comma_pos) = new_start_str.find(',') {
                                        if let Ok(start) = new_start_str[..comma_pos].parse::<i32>()
                                        {
                                            new_line_num = start;
                                        }
                                    } else if let Ok(start) = new_start_str.parse::<i32>() {
                                        new_line_num = start;
                                    }
                                }
                            }
                        }

                        result.push_str(&format!(
                            "{} {} {} {} {}\n",
                            style("📍").fg(Color::Cyan),
                            style("行号范围:").fg(Color::Cyan).bold(),
                            style(format!("旧文件:{old_line_num}"))
                                .fg(Color::Red)
                                .bold(),
                            style("→").fg(Color::White),
                            style(format!("新文件:{new_line_num}"))
                                .fg(Color::Green)
                                .bold()
                        ));
                    } else {
                        // Other @ prefixed lines
                        result.push_str(&format!(
                            "{} {}\n",
                            style("📍").fg(Color::Cyan),
                            style(content.trim()).fg(Color::Cyan).bold()
                        ));
                    }
                }
                '+' => {
                    // Added line
                    if let Some(stats) = file_stats.get_mut(&current_file) {
                        stats.0 += 1;
                    }
                    if hunk_initialized {
                        result.push_str(&format!(
                            "{} {} {}",
                            style(format!("{:>4}", "")).fg(Color::White).dim(),
                            style(format!("{new_line_num:>4}")).fg(Color::Green).bold(),
                            style(format!("+ {content}")).fg(Color::Green)
                        ));
                        new_line_num += 1;
                    } else {
                        result.push_str(&format!("+ {}", style(content).fg(Color::Green)));
                    }
                }
                '-' => {
                    // Deleted line
                    if let Some(stats) = file_stats.get_mut(&current_file) {
                        stats.1 += 1;
                    }
                    if hunk_initialized {
                        result.push_str(&format!(
                            "{} {} {}",
                            style(format!("{old_line_num:>4}")).fg(Color::Red).bold(),
                            style(format!("{:>4}", "")).fg(Color::White).dim(),
                            style(format!("- {content}")).fg(Color::Red)
                        ));
                        old_line_num += 1;
                    } else {
                        result.push_str(&format!("- {}", style(content).fg(Color::Red)));
                    }
                }
                ' ' => {
                    // Context line
                    if hunk_initialized {
                        result.push_str(&format!(
                            "{} {} {}",
                            style(format!("{old_line_num:>4}")).fg(Color::White).dim(),
                            style(format!("{new_line_num:>4}")).fg(Color::White).dim(),
                            style(format!("  {content}")).dim()
                        ));
                        old_line_num += 1;
                        new_line_num += 1;
                    } else {
                        result.push_str(&format!("  {}", style(content).dim()));
                    }
                }
                _ => {
                    // Other origins - skip
                    result.push_str("");
                }
            }
            true
        })
        .map_err(CheckpointError::GitOperationFailed)?;

        // Process remaining pending changes
        if in_newline_context && (!pending_deletions.is_empty() || !pending_additions.is_empty()) {
            self.handle_remaining_newline_changes(
                &mut result,
                &pending_deletions,
                &pending_additions,
                &mut file_stats,
                &current_file,
            );
        }

        if result.is_empty() {
            return Ok(format!(
                "{} {}\n",
                style("ℹ️").fg(Color::Blue),
                style("没有发现文件差异").fg(Color::Yellow)
            ));
        }

        // Add statistics summary
        let summary = self.generate_diff_summary(&file_stats);
        Ok(format!("{result}{summary}"))
    }

    /// Helper method to format pending changes
    fn format_pending_changes(
        &self,
        result: &mut String,
        pending_deletions: &[(String, i32)],
        pending_additions: &[(String, i32)],
        file_stats: &mut HashMap<String, (i32, i32)>,
        current_file: &str,
    ) {
        for (del_content, del_line) in pending_deletions {
            if let Some(stats) = file_stats.get_mut(current_file) {
                stats.1 += 1;
            }
            result.push_str(&format!(
                "{} {} {}\n",
                style(format!("{del_line:>4}")).fg(Color::Red).bold(),
                style(format!("{:>4}", "")).fg(Color::White).dim(),
                style(format!("- {del_content}")).fg(Color::Red)
            ));
        }
        for (add_content, add_line) in pending_additions {
            if let Some(stats) = file_stats.get_mut(current_file) {
                stats.0 += 1;
            }
            result.push_str(&format!(
                "{} {} {}\n",
                style(format!("{:>4}", "")).fg(Color::White).dim(),
                style(format!("{add_line:>4}")).fg(Color::Green).bold(),
                style(format!("+ {add_content}")).fg(Color::Green)
            ));
        }
    }

    /// Handle pending newline-related changes with intelligent processing
    fn handle_pending_newline_changes(
        &self,
        result: &mut String,
        pending_deletions: &[(String, i32)],
        pending_additions: &[(String, i32)],
        file_stats: &mut HashMap<String, (i32, i32)>,
        current_file: &str,
    ) {
        // Intelligently handle newline-related changes
        if pending_deletions.len() == 1 && pending_additions.len() == 2 {
            let (del_content, del_line) = &pending_deletions[0];
            let (add1_content, add1_line) = &pending_additions[0];
            let (add2_content, add2_line) = &pending_additions[1];

            // Check if it's: delete "content" -> add "content\n" + add "new_content"
            if del_content.trim() == add1_content.trim() {
                // Show as context line + new line, not delete+add
                result.push_str(&format!(
                    "{} {} {}\n",
                    style(format!("{del_line:>4}")).fg(Color::White).dim(),
                    style(format!("{add1_line:>4}")).fg(Color::White).dim(),
                    style(format!("  {}", del_content.trim())).dim()
                ));

                // Show the new line
                if let Some(stats) = file_stats.get_mut(current_file) {
                    stats.0 += 1;
                }
                result.push_str(&format!(
                    "{} {} {}\n",
                    style(format!("{:>4}", "")).fg(Color::White).dim(),
                    style(format!("{add2_line:>4}")).fg(Color::Green).bold(),
                    style(format!("+ {}", add2_content.trim())).fg(Color::Green)
                ));
            } else {
                // Cannot intelligently handle, fall back to original display
                self.format_pending_changes(
                    result,
                    pending_deletions,
                    pending_additions,
                    file_stats,
                    current_file,
                );
            }
        } else {
            // Other cases, fall back to original display
            self.format_pending_changes(
                result,
                pending_deletions,
                pending_additions,
                file_stats,
                current_file,
            );
        }
    }

    /// Handle remaining newline-related changes with intelligent processing
    fn handle_remaining_newline_changes(
        &self,
        result: &mut String,
        pending_deletions: &[(String, i32)],
        pending_additions: &[(String, i32)],
        file_stats: &mut HashMap<String, (i32, i32)>,
        current_file: &str,
    ) {
        if pending_deletions.len() == 1 && pending_additions.len() == 2 {
            let (del_content, del_line) = &pending_deletions[0];
            let (add1_content, add1_line) = &pending_additions[0];
            let (add2_content, add2_line) = &pending_additions[1];

            // Check if it's: delete "content" -> add "content\n" + add "new_content"
            if del_content.trim() == add1_content.trim() {
                // Show as context line + new line, not delete+add
                result.push_str(&format!(
                    "{} {} {}\n",
                    style(format!("{del_line:>4}")).fg(Color::White).dim(),
                    style(format!("{add1_line:>4}")).fg(Color::White).dim(),
                    style(format!("  {}", del_content.trim())).dim()
                ));

                // Show the new line
                if let Some(stats) = file_stats.get_mut(current_file) {
                    stats.0 += 1;
                }
                result.push_str(&format!(
                    "{} {} {}\n",
                    style(format!("{:>4}", "")).fg(Color::White).dim(),
                    style(format!("{add2_line:>4}")).fg(Color::Green).bold(),
                    style(format!("+ {}", add2_content.trim())).fg(Color::Green)
                ));
            } else {
                // Cannot intelligently handle, fall back to original display
                self.format_pending_changes(
                    result,
                    pending_deletions,
                    pending_additions,
                    file_stats,
                    current_file,
                );
            }
        } else if pending_deletions.len() == 2 && pending_additions.len() == 1 {
            // Handle "delete multiple lines + add 1 line" case (like removing newline)
            let (del1_content, del1_line) = &pending_deletions[0];
            let (del2_content, del2_line) = &pending_deletions[1];
            let (add_content, add_line) = &pending_additions[0];

            println!(
                "🔍 比较内容: del1='{}' + del2='{}' vs add='{}'",
                del1_content.trim(),
                del2_content.trim(),
                add_content.trim()
            );

            // Check if it's: delete "content1\n" + delete "content2" -> add "content1" (remove second line)
            if del1_content.trim() == add_content.trim() {
                println!("🔍 智能优化生效（删除换行符）！");

                // Show as removing second line, first line remains unchanged
                result.push_str(&format!(
                    "{} {} {}\n",
                    style(format!("{del1_line:>4}")).fg(Color::White).dim(),
                    style(format!("{add_line:>4}")).fg(Color::White).dim(),
                    style(format!("  {}", del1_content.trim())).dim()
                ));

                // Show deleted second line
                if let Some(stats) = file_stats.get_mut(current_file) {
                    stats.1 += 1;
                }
                result.push_str(&format!(
                    "{} {} {}\n",
                    style(format!("{del2_line:>4}")).fg(Color::Red).bold(),
                    style(format!("{:>4}", "")).fg(Color::White).dim(),
                    style(format!("- {}", del2_content.trim())).fg(Color::Red)
                ));
            } else {
                // Cannot intelligently handle, fall back to original display
                self.format_pending_changes(
                    result,
                    pending_deletions,
                    pending_additions,
                    file_stats,
                    current_file,
                );
            }
        } else {
            // Other cases, fall back to original display
            self.format_pending_changes(
                result,
                pending_deletions,
                pending_additions,
                file_stats,
                current_file,
            );
        }
    }

    /// Generate a summary of diff statistics
    fn generate_diff_summary(&self, file_stats: &HashMap<String, (i32, i32)>) -> String {
        let mut summary = String::new();
        summary.push_str(&format!(
            "\n{}\n",
            style("─".repeat(80)).fg(Color::Blue).dim()
        ));

        let total_files = file_stats.len();
        let total_additions: i32 = file_stats.values().map(|(a, _)| *a).sum();
        let total_deletions: i32 = file_stats.values().map(|(_, d)| *d).sum();

        summary.push_str(&format!(
            "{} {} {} 个文件变更",
            style("📊").fg(Color::Blue),
            style("统计:").fg(Color::White).bold(),
            style(total_files).fg(Color::Cyan).bold()
        ));

        if total_additions > 0 || total_deletions > 0 {
            summary.push_str(", ");
            if total_additions > 0 {
                summary.push_str(&format!(
                    "{} {} 行新增",
                    style("+").fg(Color::Green).bold(),
                    style(total_additions).fg(Color::Green).bold()
                ));
            }
            if total_additions > 0 && total_deletions > 0 {
                summary.push_str(", ");
            }
            if total_deletions > 0 {
                summary.push_str(&format!(
                    "{} {} 行删除",
                    style("-").fg(Color::Red).bold(),
                    style(total_deletions).fg(Color::Red).bold()
                ));
            }
        }
        summary.push('\n');

        // Add line number explanation
        summary.push_str(&format!(
            "{} {} {} {} {}\n",
            style("💡").fg(Color::Yellow),
            style("行号格式:").fg(Color::White).bold(),
            style("旧行号").fg(Color::Red),
            style("新行号").fg(Color::Green),
            style("内容").fg(Color::White)
        ));

        summary
    }
}
