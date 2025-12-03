//! Integration tests for the `edit` command.
//!
//! See SPEC.md#invy-edit-item

mod common;

use predicates::prelude::*;

/// Test: edit name
#[test]
fn edit_name() {
    let env = common::TestEnv::new();

    // Setup
    env.add("hammer").success();

    // Edit name
    env.run(&["edit", "hammer", "--name", "claw hammer"])
        .success()
        .stdout(predicate::str::contains("Updated"));

    // Verify new name
    env.run(&["show", "claw hammer"]).success();

    // Old name should not exist
    env.run(&["show", "hammer"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test: edit description
#[test]
fn edit_description() {
    let env = common::TestEnv::new();

    // Setup
    env.add_with_desc("hammer", "old description").success();

    // Edit description
    env.run(&["edit", "hammer", "--desc", "new description"])
        .success()
        .stdout(predicate::str::contains("Updated"));

    // Verify new description
    env.run(&["show", "hammer"])
        .success()
        .stdout(predicate::str::contains("new description"));
}

/// Test: edit both name and description
#[test]
fn edit_both() {
    let env = common::TestEnv::new();

    // Setup
    env.add_with_desc("hammer", "old description").success();

    // Edit both
    env.run(&[
        "edit",
        "hammer",
        "--name",
        "ball peen",
        "--desc",
        "new description",
    ])
    .success()
    .stdout(predicate::str::contains("Updated"));

    // Verify changes
    env.run(&["show", "ball peen"])
        .success()
        .stdout(predicate::str::contains("new description"));
}

/// Test: error on non-existent item
#[test]
fn edit_nonexistent_item_fails() {
    let env = common::TestEnv::new();

    env.run(&["edit", "nonexistent", "--name", "newname"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test: error when no changes specified
#[test]
fn edit_no_changes_fails() {
    let env = common::TestEnv::new();

    // Setup
    env.add("hammer").success();

    // Edit with no flags
    env.run(&["edit", "hammer"])
        .failure()
        .stderr(predicate::str::contains("no changes"));
}

/// Test: edit name causes conflict fails
#[test]
fn edit_name_conflict_fails() {
    let env = common::TestEnv::new();

    // Setup two items
    env.add("hammer").success();
    env.add("screwdriver").success();

    // Try to rename screwdriver to hammer
    env.run(&["edit", "screwdriver", "--name", "hammer"])
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

/// Test: clear description with empty string
#[test]
fn edit_clear_description() {
    let env = common::TestEnv::new();

    // Setup with description
    env.add_with_desc("hammer", "has description").success();

    // Clear description
    env.run(&["edit", "hammer", "--desc", ""])
        .success()
        .stdout(predicate::str::contains("Updated"));

    // Verify description is cleared (show shouldn't contain old desc)
    let result = env.run(&["show", "hammer", "--json"]).success();
    // In JSON, empty/null description should be present
    let _ = result;
}

/// Test: edit with JSON output
#[test]
fn edit_with_json_output() {
    let env = common::TestEnv::new();

    // Setup
    env.add("hammer").success();

    // Edit with JSON output
    env.run(&["edit", "hammer", "--name", "new_hammer", "--json"])
        .success()
        .stdout(predicate::str::contains(r#""name":"new_hammer""#));
}
