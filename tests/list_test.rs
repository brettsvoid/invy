//! Integration tests for the `list` command.
//!
//! See SPEC.md#invy-list-place

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

/// Test: list items in specific place
#[test]
fn list_items_in_place() {
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
        .stdout(predicate::str::contains(
            "id,name,description,kind,child_count",
        ));
}

/// Test: list empty place
#[test]
fn list_empty_place() {
    let env = common::TestEnv::new();

    // Setup empty place
    env.add("empty_box").success();

    // List should succeed but show nothing (or empty message)
    env.run(&["list", "empty_box"]).success();
}

/// Test: list non-existent place fails
#[test]
fn list_nonexistent_place_fails() {
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

/// Test: list shows child count for places
#[test]
fn list_shows_child_count() {
    let env = common::TestEnv::new();

    // Setup place with items
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
        .stdout(
            predicate::str::contains("├── screwdriver")
                .or(predicate::str::contains("└── screwdriver")),
        );
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

    assert!(alpha_pos < middle_pos, "alpha should come before middle");
    assert!(middle_pos < zebra_pos, "middle should come before zebra");
}

/// Test: duplicates share one row, and an item that differs gets its own
#[test]
fn list_groups_duplicates() {
    let env = common::TestEnv::new();
    env.run(&["add", "hdmi cable", "--in", "storage", "--count", "3"])
        .success();
    env.run(&["add", "hdmi cable", "--in", "storage", "--desc", "2m"])
        .success();

    env.run(&["list", "storage"])
        .success()
        .stdout(predicate::str::contains("hdmi cable ×3"))
        .stdout(predicate::str::contains("2m"))
        .stdout(predicate::function(|out: &str| out.lines().count() == 3));
}

/// Test: JSON keeps one object per item
#[test]
fn list_json_does_not_group() {
    let env = common::TestEnv::new();
    env.run(&["add", "hdmi cable", "--in", "storage", "--count", "3"])
        .success();

    env.run(&["list", "storage", "--json"])
        .success()
        .stdout(predicate::function(|out: &str| {
            out.matches(r#""id":"#).count() == 3
        }));
}

/// Test: the recursive tree groups duplicates too, with an ASCII x for ascii
#[test]
fn list_recursive_groups_duplicates() {
    let env = common::TestEnv::new();
    env.run(&["add", "hdmi cable", "--in", "storage", "--count", "2"])
        .success();

    env.run(&["list", "--recursive", "--glyphs", "unicode"])
        .success()
        .stdout(predicate::str::contains("└── hdmi cable ×2"));
    env.run(&["list", "--recursive", "--glyphs", "ascii"])
        .success()
        .stdout(predicate::str::contains("`-- hdmi cable x2"));
}

/// Test: two same-named places are not grouped once one holds something
#[test]
fn list_does_not_group_a_place_that_holds_things() {
    let env = common::TestEnv::new();
    env.run(&["add", "box", "--in", "wardrobe", "--count", "2"])
        .success();
    env.run(&["add", "phone", "--in", "@2"]).success();

    env.run(&["list", "wardrobe"])
        .success()
        .stdout(predicate::str::contains("×").not())
        .stdout(predicate::function(|out: &str| out.lines().count() == 3));
}

/// Test: things at root get their own Unsorted section, after the places
#[test]
fn list_puts_unsorted_things_in_their_own_section() {
    let env = common::TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();
    env.add("hammer").success();
    env.add("tape").success();

    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("Unsorted (2)"))
        .stdout(predicate::function(|out: &str| {
            let garage = out.find("garage");
            let heading = out.find("Unsorted");
            let hammer = out.find("hammer");
            matches!((garage, heading, hammer), (Some(g), Some(u), Some(h)) if g < u && u < h)
        }));
}

/// Test: no Unsorted section when every root item is a place
#[test]
fn list_without_unsorted_things_has_no_section() {
    let env = common::TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();

    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("Unsorted").not());
}

/// Test: JSON keeps one flat list of root items
#[test]
fn list_json_has_no_unsorted_section() {
    let env = common::TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();
    env.add("hammer").success();

    env.run(&["list", "--json"])
        .success()
        .stdout(predicate::str::contains("Unsorted").not())
        .stdout(predicate::function(|out: &str| {
            out.matches(r#""id":"#).count() == 2
        }));
}

/// Test: the recursive tree puts unsorted things under a heading too
#[test]
fn list_recursive_puts_unsorted_things_under_a_heading() {
    let env = common::TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();
    env.add("hammer").success();

    env.run(&["list", "--recursive", "--glyphs", "unicode"])
        .success()
        .stdout(predicate::str::contains("Unsorted (1)\n└── hammer"));
}

/// Test: output cut short by a closed pipe, as with `| head`, is not a panic
#[test]
fn list_into_a_closed_pipe_does_not_panic() {
    use std::process::Stdio;

    let env = common::TestEnv::new();
    // Enough JSON to overflow a pipe buffer, so a write has to fail.
    env.run(&["add", "a cable with a fairly long name", "--count", "3000"])
        .success();

    let mut child = env
        .std_cmd()
        .args(["list", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
}

/// Test: a flat list is in name order, ignoring case
#[test]
fn list_is_in_name_order() {
    let env = common::TestEnv::new();
    env.add("zebra").success();
    env.add("apple").success();
    env.add("Mango").success();

    env.run(&["list"])
        .success()
        .stdout(predicate::function(|out: &str| {
            let apple = out.find("apple");
            let mango = out.find("Mango");
            let zebra = out.find("zebra");
            matches!((apple, mango, zebra), (Some(a), Some(m), Some(z)) if a < m && m < z)
        }));
}
