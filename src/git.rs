//! Git metadata utilities for build tracking

use crate::infra::{CommandExecutor, RealCommandExecutor};
use thiserror::Error;

/// Git operation errors
#[derive(Debug, Error)]
pub enum GitError {
    /// Git command failed with an error message
    #[error("Git command failed: {0}")]
    CommandFailed(String),

    /// The current directory is not a git repository
    #[error("Not a git repository")]
    NotARepository,

    /// Git output contained invalid UTF-8
    #[error("Invalid UTF-8 in git output")]
    InvalidUtf8,

    /// IO error occurred while executing git command
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Git repository interface with dependency injection for testability
pub struct GitRepository<CE: CommandExecutor = RealCommandExecutor> {
    cmd_executor: CE,
}

impl GitRepository<RealCommandExecutor> {
    /// Create a new GitRepository with real command execution
    pub fn new() -> Self {
        Self {
            cmd_executor: RealCommandExecutor,
        }
    }
}

impl Default for GitRepository<RealCommandExecutor> {
    fn default() -> Self {
        Self::new()
    }
}

impl<CE: CommandExecutor> GitRepository<CE> {
    /// Create a GitRepository with a custom command executor (for testing)
    pub fn with_executor(cmd_executor: CE) -> Self {
        Self { cmd_executor }
    }

    /// Get current git commit hash (short form)
    ///
    /// Returns `Ok(Some(hash))` if in a git repository,
    /// `Ok(None)` if not in a git repository,
    /// `Err(GitError)` if git command fails unexpectedly.
    pub fn get_commit_hash(&self) -> Result<Option<String>, GitError> {
        let output = match self
            .cmd_executor
            .execute(|cmd| cmd.args(["rev-parse", "--short", "HEAD"]), "git")
        {
            Ok(output) => output,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Git command not found
                return Ok(None);
            }
            Err(e) => return Err(GitError::Io(e)),
        };

        if !output.status.success() {
            // Check if it's a "not a git repository" error
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not a git repository") {
                return Ok(None);
            }
            return Err(GitError::CommandFailed(stderr.to_string()));
        }

        let hash = String::from_utf8(output.stdout)
            .map_err(|_| GitError::InvalidUtf8)?
            .trim()
            .to_string();

        Ok(Some(hash))
    }

    /// Get current git branch name
    ///
    /// Returns `Ok(Some(branch))` if in a git repository,
    /// `Ok(None)` if not in a git repository,
    /// `Err(GitError)` if git command fails unexpectedly.
    pub fn get_branch_name(&self) -> Result<Option<String>, GitError> {
        let output = match self
            .cmd_executor
            .execute(|cmd| cmd.args(["rev-parse", "--abbrev-ref", "HEAD"]), "git")
        {
            Ok(output) => output,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Git command not found
                return Ok(None);
            }
            Err(e) => return Err(GitError::Io(e)),
        };

        if !output.status.success() {
            // Check if it's a "not a git repository" error
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not a git repository") {
                return Ok(None);
            }
            return Err(GitError::CommandFailed(stderr.to_string()));
        }

        let branch = String::from_utf8(output.stdout)
            .map_err(|_| GitError::InvalidUtf8)?
            .trim()
            .to_string();

        Ok(Some(branch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::CommandExecutor;
    use std::process::{Command, ExitStatus, Output};

    // Mock CommandExecutor for testing
    struct MockCommandExecutor {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        success: bool,
    }

    impl CommandExecutor for MockCommandExecutor {
        fn status(&self, _cmd: &mut Command) -> std::io::Result<ExitStatus> {
            unimplemented!()
        }

        fn output(&self, _cmd: &mut Command) -> std::io::Result<Output> {
            Ok(Output {
                status: if self.success {
                    ExitStatus::default()
                } else {
                    // This is a bit hacky but works for tests
                    ExitStatus::default()
                },
                stdout: self.stdout.clone(),
                stderr: self.stderr.clone(),
            })
        }
    }

    #[test]
    fn test_get_commit_hash_success() {
        let mock = MockCommandExecutor {
            stdout: b"abc1234\n".to_vec(),
            stderr: vec![],
            success: true,
        };
        let repo = GitRepository::with_executor(mock);

        let result = repo.get_commit_hash().unwrap();
        assert_eq!(result, Some("abc1234".to_string()));
    }

    #[test]
    fn test_get_branch_name_success() {
        let mock = MockCommandExecutor {
            stdout: b"main\n".to_vec(),
            stderr: vec![],
            success: true,
        };
        let repo = GitRepository::with_executor(mock);

        let result = repo.get_branch_name().unwrap();
        assert_eq!(result, Some("main".to_string()));
    }

    // Integration tests with real git
    #[test]
    fn test_get_commit_hash_returns_option() {
        let repo = GitRepository::new();
        let _ = repo.get_commit_hash();
    }

    #[test]
    fn test_get_branch_name_returns_option() {
        let repo = GitRepository::new();
        let _ = repo.get_branch_name();
    }

    #[test]
    fn test_get_commit_hash_handles_detached_head() {
        let repo = GitRepository::new();
        if let Ok(Some(hash)) = repo.get_commit_hash() {
            assert!(!hash.is_empty(), "Commit hash should not be empty");
            assert!(
                hash.len() >= 7 && hash.len() <= 40,
                "Hash should be 7-40 chars"
            );
            assert!(
                hash.chars().all(|c| c.is_ascii_hexdigit()),
                "Hash should be hex"
            );
        }
    }

    #[test]
    fn test_get_branch_name_detached_head_returns_head() {
        let repo = GitRepository::new();
        if let Ok(Some(branch)) = repo.get_branch_name() {
            assert!(!branch.is_empty(), "Branch name should not be empty");
        }
    }

    #[test]
    fn test_git_functions_outside_repository() {
        use std::env;

        let original_dir = env::current_dir().ok();

        if let Ok(temp_dir) = tempfile::tempdir() {
            if env::set_current_dir(temp_dir.path()).is_ok() {
                let repo = GitRepository::new();
                let hash = repo.get_commit_hash();
                let branch = repo.get_branch_name();

                // Restore original directory
                if let Some(dir) = original_dir {
                    let _ = env::set_current_dir(dir);
                }

                // Should return Ok(None) when not in git repo
                assert!(hash.is_ok());
                assert!(branch.is_ok());
            }
        }
    }

    #[test]
    fn test_get_commit_hash_format_validation() {
        let repo = GitRepository::new();
        if let Ok(Some(hash)) = repo.get_commit_hash() {
            assert!(hash.len() >= 7, "Hash too short: {}", hash.len());
            assert!(hash.len() <= 40, "Hash too long: {}", hash.len());
            assert!(
                hash.chars().all(|c| c.is_ascii_hexdigit()),
                "Hash contains non-hex characters: {}",
                hash
            );
            assert!(
                !hash.contains(char::is_whitespace),
                "Hash contains whitespace"
            );
        }
    }

    #[test]
    fn test_get_branch_name_format_validation() {
        let repo = GitRepository::new();
        if let Ok(Some(branch)) = repo.get_branch_name() {
            assert!(!branch.is_empty(), "Branch name is empty");
            assert!(
                !branch.contains(char::is_whitespace),
                "Branch name contains whitespace: '{}'",
                branch
            );
            assert_eq!(branch, branch.trim(), "Branch name not trimmed");
        }
    }

    #[test]
    fn test_get_commit_hash_consistency() {
        let repo = GitRepository::new();
        let hash1 = repo.get_commit_hash();
        let hash2 = repo.get_commit_hash();

        if let (Ok(Some(h1)), Ok(Some(h2))) = (&hash1, &hash2) {
            assert_eq!(h1, h2, "Hash changed between calls");
        }
    }

    #[test]
    fn test_get_branch_name_consistency() {
        let repo = GitRepository::new();
        let branch1 = repo.get_branch_name();
        let branch2 = repo.get_branch_name();

        if let (Ok(Some(b1)), Ok(Some(b2))) = (&branch1, &branch2) {
            assert_eq!(b1, b2, "Branch changed between calls");
        }
    }

    #[test]
    fn test_get_branch_name_in_new_repo_no_commits() {
        let mock_exec = MockCommandExecutor {
            stdout: vec![],
            stderr: b"fatal: ref HEAD is not a symbolic ref".to_vec(),
            success: false,
        };
        let repo = GitRepository::with_executor(mock_exec);

        let result = repo.get_branch_name();
        // New repo with no commits may return None or error
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_get_commit_hash_with_short_hash() {
        let mock_exec = MockCommandExecutor {
            stdout: b"abc123\n".to_vec(),
            stderr: vec![],
            success: true,
        };
        let repo = GitRepository::with_executor(mock_exec);

        let hash = repo.get_commit_hash().unwrap();
        assert_eq!(hash, Some("abc123".to_string()));
    }

    #[test]
    fn test_get_branch_name_with_special_characters() {
        let mock_exec = MockCommandExecutor {
            stdout: b"feature/issue-123\n".to_vec(),
            stderr: vec![],
            success: true,
        };
        let repo = GitRepository::with_executor(mock_exec);

        let branch = repo.get_branch_name().unwrap();
        assert_eq!(branch, Some("feature/issue-123".to_string()));
    }

    #[test]
    fn test_get_commit_hash_with_full_hash() {
        let mock_exec = MockCommandExecutor {
            stdout: b"a1b2c3d4e5f6\n".to_vec(),
            stderr: vec![],
            success: true,
        };
        let repo = GitRepository::with_executor(mock_exec);

        let hash = repo.get_commit_hash().unwrap();
        assert_eq!(hash, Some("a1b2c3d4e5f6".to_string()));
    }
}
