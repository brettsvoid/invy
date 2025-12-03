//! Integration tests for the `show` command.
//!
//! See SPEC.md#invy-show-item

mod common;

use predicates::prelude::*;

/// Test: show item details + full path
#[test]
fn show_item_details_and_path() {
    let env = common::TestEnv::new();

    // Setup hierarchy
    env.add("garage").success();
    env.add_into("toolbox", "garage").success();
    env.add_full("hammer", "claw hammer", "toolbox").success();

    // Show should display details and path
    env.run(&["show", "hammer"])
        .success()
        .stdout(predicate::str::contains("hammer"))
        .stdout(predicate::str::contains("claw hammer"))
        .stdout(predicate::str::contains("toolbox"))
        .stdout(predicate::str::contains("garage"));
}

/// Test: show container with contents count
#[test]
fn show_container_with_contents_count() {
    let env = common::TestEnv::new();

    // Setup container with items
    env.add("toolbox").success();
    env.add_into("hammer", "toolbox").success();
    env.add_into("screwdriver", "toolbox").success();

    // Show container should display item count
    env.run(&["show", "toolbox"])
        .success()
        .stdout(predicate::str::contains("toolbox"))
        .stdout(predicate::str::contains("2")); // Contains 2 items
}

/// Test: error on non-existent item
#[test]
fn show_nonexistent_item_fails() {
    let env = common::TestEnv::new();

    env.run(&["show", "nonexistent"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test: show with JSON output
#[test]
fn show_with_json_output() {
    let env = common::TestEnv::new();

    // Setup
    env.add_with_desc("hammer", "claw hammer").success();

    // Show with JSON
    env.run(&["show", "hammer", "--json"])
        .success()
        .stdout(predicate::str::contains(r#""name":"hammer""#))
        .stdout(predicate::str::contains(r#""description":"claw hammer""#))
        .stdout(predicate::str::contains("created_at"))
        .stdout(predicate::str::contains("updated_at"));
}

/// Test: show item by full path when name is ambiguous
#[test]
fn show_by_full_path() {
    let env = common::TestEnv::new();

    // Setup two items with same name in different containers
    env.add_with_desc("hammer", "root hammer").success();
    env.add_into("hammer", "toolbox").success();

    // Show by path should work
    env.run(&["show", "toolbox/hammer"])
        .success()
        .stdout(predicate::str::contains("hammer"));
}

/// Test: ambiguous name without path shows error
#[test]
fn show_ambiguous_name_fails() {
    let env = common::TestEnv::new();

    // Setup two items with same name
    env.add("hammer").success();
    env.add_into("hammer", "toolbox").success();

    // Show without path should fail with helpful message
    env.run(&["show", "hammer"])
        .failure()
        .stderr(predicate::str::contains("ambiguous"));
}
