//! Common test utilities and helpers.

use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test harness that provides a temporary database for each test.
pub struct TestEnv {
    pub temp_dir: TempDir,
    pub db_path: PathBuf,
}

impl TestEnv {
    /// Create a new test environment with a temporary database.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        Self { temp_dir, db_path }
    }

    /// Get a Command configured to use this test environment's database.
    pub fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("invy").expect("Failed to find invy binary");
        cmd.arg("--db").arg(&self.db_path);
        cmd
    }

    /// Run invy with the given arguments.
    pub fn run(&self, args: &[&str]) -> assert_cmd::assert::Assert {
        self.cmd().args(args).assert()
    }

    /// Run invy add command.
    pub fn add(&self, name: &str) -> assert_cmd::assert::Assert {
        self.run(&["add", name])
    }

    /// Run invy add command with description.
    pub fn add_with_desc(&self, name: &str, desc: &str) -> assert_cmd::assert::Assert {
        self.run(&["add", name, "--desc", desc])
    }

    /// Run invy add command into a container.
    pub fn add_into(&self, name: &str, container: &str) -> assert_cmd::assert::Assert {
        self.run(&["add", name, "--in", container])
    }

    /// Run invy add command with description into a container.
    pub fn add_full(&self, name: &str, desc: &str, container: &str) -> assert_cmd::assert::Assert {
        self.run(&["add", name, "--desc", desc, "--in", container])
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}
