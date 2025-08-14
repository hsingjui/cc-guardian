//! Shared types and constants for git operations
//!
//! This module contains common types, constants, and utilities used across
//! all git operation modules.

/// The name of the CCG (Claude Code Checkpoint Guardian) branch
///
/// This is the special branch where all checkpoints are stored.
pub const CCG_BRANCH_NAME: &str = "ccg";

/// Default commit message for initial commits
///
/// Used when creating the first commit in a new repository.
pub const DEFAULT_COMMIT_MESSAGE: &str = "Initial commit - Claude Code Checkpoint Guardian init";

/// Statistics about file differences
///
/// Contains aggregated information about changes in a diff, including
/// file counts and line change statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffStats {
    /// Total number of files changed
    pub total_files: usize,
    /// Number of lines added across all files
    pub additions: i32,
    /// Number of lines deleted across all files
    pub deletions: i32,
    /// Number of files modified (not including pure additions/deletions)
    pub modifications: i32,
}

impl DiffStats {
    /// Create a new DiffStats with all values set to zero
    pub fn new() -> Self {
        Self {
            total_files: 0,
            additions: 0,
            deletions: 0,
            modifications: 0,
        }
    }
}

impl Default for DiffStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a single file change
///
/// Represents the changes made to a specific file in a diff,
/// including the type of change and line statistics.
#[derive(Debug, Clone)]
pub struct FileChangeInfo {
    /// Path to the changed file relative to repository root
    pub path: String,
    /// Type of change (Added, Modified, Deleted, Renamed, etc.)
    pub status: git2::Delta,
    /// Number of lines added in this file
    pub additions: i32,
    /// Number of lines deleted in this file
    pub deletions: i32,
}

impl FileChangeInfo {
    /// Create a new FileChangeInfo with zero line changes
    ///
    /// # Arguments
    /// * `path` - The file path relative to repository root
    /// * `status` - The type of change made to the file
    pub fn new(path: String, status: git2::Delta) -> Self {
        Self {
            path,
            status,
            additions: 0,
            deletions: 0,
        }
    }

    /// Create a new FileChangeInfo with line change statistics
    ///
    /// # Arguments
    /// * `path` - The file path relative to repository root
    /// * `status` - The type of change made to the file
    /// * `additions` - Number of lines added
    /// * `deletions` - Number of lines deleted
    pub fn with_stats(path: String, status: git2::Delta, additions: i32, deletions: i32) -> Self {
        Self {
            path,
            status,
            additions,
            deletions,
        }
    }
}
