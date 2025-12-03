//! Integration tests for the `add` command.
//!
//! See SPEC.md#invy-add-name

mod common;

use predicates::prelude::*;

/// Test: add item with name only
#[test]
fn add_item_with_name_only() {
    let env = common::TestEnv::new();

    env.add("hammer")
        .success()
        .stdout(predicate::str::contains("Added: hammer"));
}

/// Test: add item with description
#[test]
fn add_item_with_description() {
    let env = common::TestEnv::new();

    env.add_with_desc("hammer", "claw hammer")
        .success()
        .stdout(predicate::str::contains("Added: hammer"));
}

/// Test: add item into container (auto-creates container)
#[test]
fn add_item_into_container_auto_creates() {
    let env = common::TestEnv::new();

    // Container "toolbox" doesn't exist yet - should be auto-created
    env.add_into("hammer", "toolbox")
        .success()
        .stdout(predicate::str::contains("Added: hammer"))
        .stdout(predicate::str::contains("toolbox"));
}

/// Test: add item into nested container path
#[test]
fn add_item_into_nested_container() {
    let env = common::TestEnv::new();

    // First create the hierarchy
    env.add("garage").success();
    env.add_into("toolbox", "garage").success();

    // Now add into the nested container
    env.add_into("hammer", "toolbox")
        .success()
        .stdout(predicate::str::contains("Added: hammer"));
}

/// Test: error on duplicate name in same container
#[test]
fn add_duplicate_name_in_same_container_fails() {
    let env = common::TestEnv::new();

    // Add first item
    env.add("hammer").success();

    // Try to add duplicate at root - should fail
    env.add("hammer")
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

/// Test: duplicate names allowed in different containers
#[test]
fn add_duplicate_name_in_different_containers_succeeds() {
    let env = common::TestEnv::new();

    // Add hammer at root
    env.add("hammer").success();

    // Add another hammer in toolbox - should succeed (different container)
    env.add_into("hammer", "toolbox").success();
}

/// Test: add with JSON output
#[test]
fn add_with_json_output() {
    let env = common::TestEnv::new();

    env.run(&["add", "hammer", "--desc", "claw hammer", "--json"])
        .success()
        .stdout(predicate::str::contains(r#""name":"hammer""#))
        .stdout(predicate::str::contains(r#""description":"claw hammer""#));
}

/// Test: add with CSV output
#[test]
fn add_with_csv_output() {
    let env = common::TestEnv::new();

    env.run(&["add", "hammer", "--csv"])
        .success()
        .stdout(predicate::str::contains("id,name,description,container"))
        .stdout(predicate::str::contains("hammer"));
}
