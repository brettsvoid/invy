//! Integration tests for the config file.
//!
//! See SPEC.md#configuration

mod common;

use predicates::prelude::*;

/// Test: without --db, the database named in the config is used
#[test]
fn config_db_is_used() {
    let env = common::TestEnv::new();
    let db = env.temp_dir.path().join("from-config.db");
    env.write_config(&format!("db = {:?}\n", db));

    env.cmd_without_db()
        .args(["add", "hammer"])
        .assert()
        .success();

    assert!(db.exists());
    assert!(!env.default_db().exists());
    env.cmd_without_db()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hammer"));
}

/// Test: --db beats the config
#[test]
fn db_flag_beats_config() {
    let env = common::TestEnv::new();
    let db = env.temp_dir.path().join("from-config.db");
    env.write_config(&format!("db = {:?}\n", db));

    env.add("hammer").success();

    assert!(env.db_path.exists());
    assert!(!db.exists());
}

/// Test: with no config file, the database is $XDG_DATA_HOME/invy/invy.db,
/// and its directory is created
#[test]
fn no_config_uses_default() {
    let env = common::TestEnv::new();

    env.cmd_without_db()
        .args(["add", "hammer"])
        .assert()
        .success();

    assert!(env.default_db().exists());
}

/// Test: without XDG_DATA_HOME, the database is ~/.local/share/invy/invy.db
#[test]
fn default_db_falls_back_to_home_local_share() {
    let env = common::TestEnv::new();

    env.cmd_without_db()
        .env_remove("XDG_DATA_HOME")
        .args(["add", "hammer"])
        .assert()
        .success();

    assert!(env.home().join(".local/share/invy/invy.db").exists());
}

/// Test: a config without `db` leaves the default in place
#[test]
fn config_without_db_uses_default() {
    let env = common::TestEnv::new();
    env.write_config("");

    env.cmd_without_db()
        .args(["add", "hammer"])
        .assert()
        .success();

    assert!(env.default_db().exists());
}

/// Test: a directory named by the config is not created
#[test]
fn config_db_directory_is_not_created() {
    let env = common::TestEnv::new();
    let db = env.temp_dir.path().join("missing/inventory.db");
    env.write_config(&format!("db = {:?}\n", db));

    env.cmd_without_db()
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to open database"));

    assert!(!db.parent().unwrap().exists());
}

/// Test: `~` in the config is the home directory
#[test]
fn config_db_expands_tilde() {
    let env = common::TestEnv::new();
    env.write_config("db = \"~/inventory.db\"\n");

    env.cmd_without_db()
        .args(["add", "hammer"])
        .assert()
        .success();

    assert!(env.home().join("inventory.db").exists());
}

/// Test: a relative path is taken from the config directory
#[test]
fn config_db_relative_to_config_dir() {
    let env = common::TestEnv::new();
    env.write_config("db = \"inventory.db\"\n");

    env.cmd_without_db()
        .args(["add", "hammer"])
        .assert()
        .success();

    assert!(env.config_home().join("invy/inventory.db").exists());
}

/// Test: without XDG_CONFIG_HOME, the config is read from ~/.config/invy
#[test]
fn config_falls_back_to_home_dot_config() {
    let env = common::TestEnv::new();
    let dir = env.home().join(".config/invy");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.toml"), "db = \"~/inventory.db\"\n").unwrap();

    env.cmd_without_db()
        .env_remove("XDG_CONFIG_HOME")
        .args(["add", "hammer"])
        .assert()
        .success();

    assert!(env.home().join("inventory.db").exists());
}

/// Test: an unknown setting is an error that names the file
#[test]
fn unknown_setting_is_an_error() {
    let env = common::TestEnv::new();
    env.write_config("database = \"inventory.db\"\n");

    env.cmd_without_db()
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("config.toml"))
        .stderr(predicate::str::contains("unknown field `database`"));
}
