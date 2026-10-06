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
        let env = Self { temp_dir, db_path };
        std::fs::create_dir_all(env.home()).expect("Failed to create home dir");
        env
    }

    /// Home directory for this environment, inside the temp dir.
    pub fn home(&self) -> PathBuf {
        self.temp_dir.path().join("home")
    }

    /// `$XDG_CONFIG_HOME` for this environment. invy reads `invy/config.toml`
    /// under it.
    pub fn config_home(&self) -> PathBuf {
        self.temp_dir.path().join("config")
    }

    /// `$XDG_DATA_HOME` for this environment.
    pub fn data_home(&self) -> PathBuf {
        self.temp_dir.path().join("data")
    }

    /// Where invy puts the database when nothing names one.
    pub fn default_db(&self) -> PathBuf {
        self.data_home().join("invy/invy.db")
    }

    /// Write `invy/config.toml` under `config_home`.
    pub fn write_config(&self, text: &str) {
        let dir = self.config_home().join("invy");
        std::fs::create_dir_all(&dir).expect("Failed to create config dir");
        std::fs::write(dir.join("config.toml"), text).expect("Failed to write config");
    }

    /// Get a Command isolated from the real home and config, without `--db`.
    pub fn cmd_without_db(&self) -> Command {
        let mut cmd = Command::cargo_bin("invy").expect("Failed to find invy binary");
        // An ambient INVY_GLYPHS would change the output these tests assert on.
        cmd.env_remove("INVY_GLYPHS");
        // Keep the user's own config and database out of reach.
        cmd.env("HOME", self.home());
        cmd.env("XDG_CONFIG_HOME", self.config_home());
        cmd.env("XDG_DATA_HOME", self.data_home());
        cmd
    }

    /// Get a Command configured to use this test environment's database.
    pub fn cmd(&self) -> Command {
        let mut cmd = self.cmd_without_db();
        cmd.arg("--db").arg(&self.db_path);
        cmd
    }

    /// A plain `std::process::Command` set up like `cmd()`, for tests that
    /// need control over the child's pipes.
    pub fn std_cmd(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_invy"));
        cmd.env_remove("INVY_GLYPHS");
        cmd.env("HOME", self.home());
        cmd.env("XDG_CONFIG_HOME", self.config_home());
        cmd.env("XDG_DATA_HOME", self.data_home());
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

    /// Run invy add command into a place.
    pub fn add_into(&self, name: &str, place: &str) -> assert_cmd::assert::Assert {
        self.run(&["add", name, "--in", place])
    }

    /// Run invy add command with description into a place.
    pub fn add_full(&self, name: &str, desc: &str, place: &str) -> assert_cmd::assert::Assert {
        self.run(&["add", name, "--desc", desc, "--in", place])
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}
