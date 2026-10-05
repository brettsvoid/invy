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

/// Test: show place with contents count
#[test]
fn show_place_with_contents_count() {
    let env = common::TestEnv::new();

    // Setup place with items
    env.add("toolbox").success();
    env.add_into("hammer", "toolbox").success();
    env.add_into("screwdriver", "toolbox").success();

    // Show place should display item count
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

    // Setup two items with same name in different places
    env.add_with_desc("hammer", "root hammer").success();
    env.add_into("hammer", "toolbox").success();

    // Show by path should work
    env.run(&["show", "toolbox/hammer"])
        .success()
        .stdout(predicate::str::contains("hammer"));
}

/// Test: nonexistent item suggests substring matches
#[test]
fn show_nonexistent_item_suggests_matches() {
    let env = common::TestEnv::new();

    // Setup two items that match the substring "kvm"
    env.add_with_desc("8k displayport kvm switch", "dual monitor, 2x2")
        .success();
    env.add("usb kvm switch").success();

    // show with a string that doesn't exact-match either name
    env.run(&["show", "kvm switch"])
        .failure()
        .stderr(predicate::str::contains("not found"))
        .stderr(predicate::str::contains("Did you mean"))
        .stderr(predicate::str::contains("8k displayport kvm switch"))
        .stderr(predicate::str::contains("usb kvm switch"));
}

/// Test: ambiguous name without path shows error
#[test]
fn show_ambiguous_name_fails() {
    let env = common::TestEnv::new();

    // Setup two items with same name
    env.add("hammer").success();
    env.add_into("hammer", "toolbox").success();

    // Show without path should fail and list both candidates
    env.run(&["show", "hammer"])
        .failure()
        .stderr(predicate::str::contains("ambiguous"))
        .stderr(predicate::str::contains("@1  hammer"))
        .stderr(predicate::str::contains("@3  toolbox/hammer"));
}

/// Test: an @id names one item
#[test]
fn show_by_id() {
    let env = common::TestEnv::new();
    env.add_with_desc("hammer", "claw").success();

    env.run(&["show", "@1"])
        .success()
        .stdout(predicate::str::contains("claw"));
}

/// Test: an @id that names nothing is not found
#[test]
fn show_unknown_id_fails() {
    let env = common::TestEnv::new();

    env.run(&["show", "@7"])
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test: a name shared by duplicates shows one of them
#[test]
fn show_one_of_several_duplicates() {
    let env = common::TestEnv::new();
    env.add("hdmi cable").success();
    env.add("hdmi cable").success();

    env.run(&["show", "hdmi cable"]).success();
}

/// Test: the same name in the same place with different descriptions is
/// ambiguous, and the error tells them apart by @id
#[test]
fn show_same_path_different_items_is_ambiguous() {
    let env = common::TestEnv::new();
    env.add_with_desc("pi power supply", "5V 3A").success();
    env.add_with_desc("pi power supply", "5V 5A").success();

    env.run(&["show", "pi power supply"])
        .failure()
        .stderr(predicate::str::contains("@1  pi power supply  5V 3A"))
        .stderr(predicate::str::contains("@2  pi power supply  5V 5A"));
}

/// Test: names match without regard to case
#[test]
fn show_ignores_case() {
    let env = common::TestEnv::new();
    env.add_into("HDMI cable", "Cable Storage").success();

    env.run(&["show", "hdmi cable"]).success();
    env.run(&["show", "cable storage/hdmi CABLE"]).success();
}

/// Test: show says how many duplicates share the place
#[test]
fn show_counts_duplicates() {
    let env = common::TestEnv::new();
    env.run(&["add", "hdmi cable", "--count", "3"]).success();
    env.add("hammer").success();

    env.run(&["show", "hdmi cable"])
        .success()
        .stdout(predicate::str::contains("Duplicates:  3 here"));
    env.run(&["show", "hammer"])
        .success()
        .stdout(predicate::str::contains("Duplicates").not());
}
