//! Integration tests for the `list` command.
//!
//! See SPEC.md#invy-list-container

mod common;

use predicates::prelude::*;

/// Test: list all top-level items
#[test]
fn list_all_top_level_items() {
    let env = common::TestEnv::new();

    // Setup
    env.add("hammer").success();
    env.add("screwdriver").success();
    env.add("wrench").success();

    // List should show all three
    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("hammer"))
        .stdout(predicate::str::contains("screwdriver"))
        .stdout(predicate::str::contains("wrench"));
}

/// Test: list items in specific container
#[test]
fn list_items_in_container() {
    let env = common::TestEnv::new();

    // Setup: toolbox with items
    env.add("toolbox").success();
    env.add_into("hammer", "toolbox").success();
    env.add_into("screwdriver", "toolbox").success();
    // Add item at root (should not appear in list toolbox)
    env.add("standalone").success();

    // List toolbox contents
    env.run(&["list", "toolbox"])
        .success()
        .stdout(predicate::str::contains("hammer"))
        .stdout(predicate::str::contains("screwdriver"))
        .stdout(predicate::str::contains("standalone").not());
}

/// Test: list with --json output
#[test]
fn list_with_json_output() {
    let env = common::TestEnv::new();

    // Setup
    env.add_with_desc("hammer", "claw hammer").success();

    // List with JSON
    env.run(&["list", "--json"])
        .success()
        .stdout(predicate::str::contains(r#""name":"hammer""#));
}

/// Test: list with --csv output
#[test]
fn list_with_csv_output() {
    let env = common::TestEnv::new();

    // Setup
    env.add("hammer").success();

    // List with CSV
    env.run(&["list", "--csv"])
        .success()
        .stdout(predicate::str::contains("id,name,description,child_count"));
}

/// Test: list empty container
#[test]
fn list_empty_container() {
    let env = common::TestEnv::new();

    // Setup empty container
    env.add("empty_box").success();

    // List should succeed but show nothing (or empty message)
    env.run(&["list", "empty_box"]).success();
}

/// Test: list non-existent container fails
#[test]
fn list_nonexistent_container_fails() {
    let env = common::TestEnv::new();

    env.run(&["list", "nonexistent"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test: list with recursive flag
#[test]
fn list_recursive() {
    let env = common::TestEnv::new();

    // Setup nested hierarchy
    env.add("garage").success();
    env.add_into("toolbox", "garage").success();
    env.add_into("hammer", "toolbox").success();

    // Recursive list should show everything
    env.run(&["list", "--recursive"])
        .success()
        .stdout(predicate::str::contains("garage"))
        .stdout(predicate::str::contains("toolbox"))
        .stdout(predicate::str::contains("hammer"));
}

/// Test: list shows child count for containers
#[test]
fn list_shows_child_count() {
    let env = common::TestEnv::new();

    // Setup container with items
    env.add("toolbox").success();
    env.add_into("hammer", "toolbox").success();
    env.add_into("screwdriver", "toolbox").success();

    // List should show toolbox with item count
    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("toolbox"))
        .stdout(predicate::str::contains("2")); // 2 items
}

/// Test: recursive list shows tree structure with Unicode characters
#[test]
fn list_recursive_shows_tree_structure() {
    let env = common::TestEnv::new();

    // Setup nested hierarchy
    env.add("garage").success();
    env.add_into("toolbox", "garage").success();
    env.add_into("hammer", "garage/toolbox").success();
    env.add_into("screwdriver", "garage/toolbox").success();

    // Recursive list should show tree with proper indentation
    env.run(&["list", "--recursive"])
        .success()
        .stdout(predicate::str::contains("garage"))
        .stdout(predicate::str::contains("└── toolbox"))
        .stdout(predicate::str::contains("├── hammer").or(predicate::str::contains("└── hammer")))
        .stdout(predicate::str::contains("├── screwdriver").or(predicate::str::contains("└── screwdriver")));
}

/// Test: recursive list shows child counts in brackets
#[test]
fn list_recursive_shows_child_counts() {
    let env = common::TestEnv::new();

    env.add("garage").success();
    env.add_into("toolbox", "garage").success();
    env.add_into("hammer", "garage/toolbox").success();

    // Should show [1] for garage (has toolbox) and [1] for toolbox (has hammer)
    env.run(&["list", "--recursive"])
        .success()
        .stdout(predicate::str::contains("garage [1]"))
        .stdout(predicate::str::contains("toolbox [1]"));
}

/// Test: recursive list with JSON stays flat
#[test]
fn list_recursive_json_is_flat_array() {
    let env = common::TestEnv::new();

    env.add("garage").success();
    env.add_into("toolbox", "garage").success();

    // JSON output should be an array with both items
    env.run(&["list", "--recursive", "--json"])
        .success()
        .stdout(predicate::str::contains(r#""name":"garage""#))
        .stdout(predicate::str::contains(r#""name":"toolbox""#));
}

/// Test: recursive list sorts alphabetically
#[test]
fn list_recursive_alphabetical_order() {
    let env = common::TestEnv::new();

    // Add items in non-alphabetical order
    env.add("zebra").success();
    env.add("alpha").success();
    env.add("middle").success();

    // Output should be sorted: alpha, middle, zebra
    let output = env
        .run(&["list", "--recursive"])
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    let alpha_pos = output_str.find("alpha").expect("alpha not found");
    let middle_pos = output_str.find("middle").expect("middle not found");
    let zebra_pos = output_str.find("zebra").expect("zebra not found");

    assert!(
        alpha_pos < middle_pos,
        "alpha should come before middle"
    );
    assert!(
        middle_pos < zebra_pos,
        "middle should come before zebra"
    );
}
