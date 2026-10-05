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

/// Test: remove empty place
#[test]
fn remove_empty_place() {
    let env = common::TestEnv::new();

    // Setup empty place
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

/// Test: removing a place leaves its contents at root, unsorted
#[test]
fn remove_place_leaves_contents_unsorted() {
    let env = common::TestEnv::new();

    // Setup place with items
    env.add("toolbox").success();
    env.add_into("hammer", "toolbox").success();
    env.add_into("screwdriver", "toolbox").success();

    // Remove place
    env.run(&["rm", "toolbox"])
        .success()
        .stdout(predicate::str::contains("Removed"))
        .stdout(predicate::str::contains("Now unsorted:"))
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

/// Test: removing a place in the middle of the tree sends its contents to root
#[test]
fn remove_nested_place_sends_contents_to_root() {
    let env = common::TestEnv::new();

    // Setup: garage -> toolbox -> hammer
    env.add("garage").success();
    env.add_into("toolbox", "garage").success();
    env.add_into("hammer", "toolbox").success();

    // Remove toolbox (middle of hierarchy)
    env.run(&["rm", "toolbox"])
        .success()
        .stdout(predicate::str::contains("Now unsorted:"));

    // Hammer should now be at root, not in garage
    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("hammer"));

    // Garage should still exist
    env.run(&["show", "garage"]).success();
}

/// Test: removing one of several duplicates leaves the rest
#[test]
fn remove_one_duplicate() {
    let env = common::TestEnv::new();
    env.add("hdmi cable").success();
    env.add("hdmi cable").success();

    env.run(&["rm", "hdmi cable"])
        .success()
        .stdout(predicate::str::contains("Removed 1 of 2: hdmi cable"));

    env.run(&["list", "--csv"])
        .success()
        .stdout(predicate::function(|out: &str| out.lines().count() == 2));
}

/// Test: `--all` removes every duplicate
#[test]
fn remove_all_duplicates() {
    let env = common::TestEnv::new();
    for _ in 0..3 {
        env.add("hdmi cable").success();
    }

    env.run(&["rm", "hdmi cable", "--all"])
        .success()
        .stdout(predicate::str::contains("Removed 3: hdmi cable"));

    env.run(&["list", "--csv"])
        .success()
        .stdout(predicate::function(|out: &str| out.lines().count() == 1));
}

/// Test: a place inside a removed place goes to root as a place, not unsorted
#[test]
fn remove_place_says_which_contents_are_places() {
    let env = common::TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();
    env.run(&["add", "toolbox", "--in", "garage", "--kind", "box"])
        .success();
    env.add_into("bike", "garage").success();

    env.run(&["rm", "garage"])
        .success()
        .stdout(predicate::str::contains("Now unsorted:\n  - bike"))
        .stdout(predicate::str::contains("Now at root:\n  - toolbox"));
}
