//! Integration tests for the `mv` command.
//!
//! See SPEC.md#invy-mv-item-destination

mod common;

use predicates::prelude::*;

/// Test: move item to different container
#[test]
fn move_item_to_different_container() {
    let env = common::TestEnv::new();

    // Setup
    env.add("toolbox").success();
    env.add("workshop").success();
    env.add_into("hammer", "toolbox").success();

    // Move hammer from toolbox to workshop
    env.run(&["mv", "hammer", "workshop"])
        .success()
        .stdout(predicate::str::contains("Moved"));

    // Verify hammer is now in workshop
    env.run(&["show", "hammer"])
        .success()
        .stdout(predicate::str::contains("workshop"));
}

/// Test: move item to top-level (no container)
#[test]
fn move_item_to_root() {
    let env = common::TestEnv::new();

    // Setup
    env.add("toolbox").success();
    env.add_into("hammer", "toolbox").success();

    // Move hammer to root
    env.run(&["mv", "hammer", "/"])
        .success()
        .stdout(predicate::str::contains("Moved"));

    // Verify hammer is at root (list root should show it)
    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("hammer"));
}

/// Test: error moving container into itself
#[test]
fn move_container_into_itself_fails() {
    let env = common::TestEnv::new();

    // Setup
    env.add("toolbox").success();

    // Try to move toolbox into itself
    env.run(&["mv", "toolbox", "toolbox"])
        .failure()
        .stderr(predicate::str::contains("cannot move"));
}

/// Test: error moving container into its descendant
#[test]
fn move_container_into_descendant_fails() {
    let env = common::TestEnv::new();

    // Setup hierarchy: garage -> shelf
    env.add("garage").success();
    env.add_into("shelf", "garage").success();

    // Try to move garage into shelf (its child)
    env.run(&["mv", "garage", "shelf"])
        .failure()
        .stderr(predicate::str::contains("cannot move"));
}

/// Test: auto-create destination container
#[test]
fn move_auto_creates_destination() {
    let env = common::TestEnv::new();

    // Setup
    env.add("hammer").success();

    // Move to container that doesn't exist
    env.run(&["mv", "hammer", "new_toolbox"])
        .success()
        .stdout(predicate::str::contains("Moved"));

    // Verify destination was created
    env.run(&["show", "new_toolbox"]).success();
}

/// Test: move non-existent item fails
#[test]
fn move_nonexistent_item_fails() {
    let env = common::TestEnv::new();

    env.run(&["mv", "nonexistent", "somewhere"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test: move causes name conflict fails
#[test]
fn move_name_conflict_fails() {
    let env = common::TestEnv::new();

    // Setup: hammer at root and in toolbox
    env.add("hammer").success();
    env.add("toolbox").success();
    env.add_into("hammer", "toolbox").success();

    // Try to move toolbox/hammer to root (conflicts with existing hammer)
    env.run(&["mv", "toolbox/hammer", "/"])
        .failure()
        .stderr(predicate::str::contains("already exists"));
}
