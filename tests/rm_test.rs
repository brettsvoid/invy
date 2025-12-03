//! Integration tests for the `rm` command.
//!
//! See SPEC.md#invy-rm-item

mod common;

use predicates::prelude::*;

/// Test: remove item
#[test]
fn remove_item() {
    let env = common::TestEnv::new();

    // Setup
    env.add("hammer").success();

    // Remove
    env.run(&["rm", "hammer"])
        .success()
        .stdout(predicate::str::contains("Removed"));

    // Verify it's gone
    env.run(&["show", "hammer"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test: remove empty container
#[test]
fn remove_empty_container() {
    let env = common::TestEnv::new();

    // Setup empty container
    env.add("empty_box").success();

    // Remove
    env.run(&["rm", "empty_box"])
        .success()
        .stdout(predicate::str::contains("Removed"));

    // Verify it's gone
    env.run(&["show", "empty_box"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test: remove non-empty container → orphans contents to top-level
#[test]
fn remove_container_orphans_contents() {
    let env = common::TestEnv::new();

    // Setup container with items
    env.add("toolbox").success();
    env.add_into("hammer", "toolbox").success();
    env.add_into("screwdriver", "toolbox").success();

    // Remove container
    env.run(&["rm", "toolbox"])
        .success()
        .stdout(predicate::str::contains("Removed"))
        .stdout(predicate::str::contains("Orphaned"))
        .stdout(predicate::str::contains("hammer"))
        .stdout(predicate::str::contains("screwdriver"));

    // Verify toolbox is gone
    env.run(&["show", "toolbox"])
        .failure()
        .stderr(predicate::str::contains("not found"));

    // Verify items are now at root
    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("hammer"))
        .stdout(predicate::str::contains("screwdriver"));
}

/// Test: error on non-existent item
#[test]
fn remove_nonexistent_item_fails() {
    let env = common::TestEnv::new();

    env.run(&["rm", "nonexistent"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test: remove nested container orphans to root
#[test]
fn remove_nested_container_orphans_to_root() {
    let env = common::TestEnv::new();

    // Setup: garage -> toolbox -> hammer
    env.add("garage").success();
    env.add_into("toolbox", "garage").success();
    env.add_into("hammer", "toolbox").success();

    // Remove toolbox (middle of hierarchy)
    env.run(&["rm", "toolbox"])
        .success()
        .stdout(predicate::str::contains("Orphaned"));

    // Hammer should now be at root, not in garage
    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("hammer"));

    // Garage should still exist
    env.run(&["show", "garage"]).success();
}
