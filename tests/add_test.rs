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

/// Test: add item into place (auto-creates place)
#[test]
fn add_item_into_place_auto_creates() {
    let env = common::TestEnv::new();

    // Place "toolbox" doesn't exist yet - should be auto-created
    env.add_into("hammer", "toolbox")
        .success()
        .stdout(predicate::str::contains("Added: hammer"))
        .stdout(predicate::str::contains("toolbox"));
}

/// Test: add item into nested place path
#[test]
fn add_item_into_nested_place() {
    let env = common::TestEnv::new();

    // First create the hierarchy
    env.add("garage").success();
    env.add_into("toolbox", "garage").success();

    // Now add into the nested place
    env.add_into("hammer", "toolbox")
        .success()
        .stdout(predicate::str::contains("Added: hammer"));
}

/// Test: the same name in the same place adds a second item
#[test]
fn add_duplicate_name_in_same_place_adds_another() {
    let env = common::TestEnv::new();

    env.add("hammer").success();
    env.add("hammer").success();

    env.run(&["list", "--csv"])
        .success()
        .stdout(predicate::function(|out: &str| out.lines().count() == 3));
}

/// Test: an item whose name matches several places goes into none of them
#[test]
fn add_into_ambiguous_place_fails() {
    let env = common::TestEnv::new();
    // Two empty boxes are duplicates, so the first one takes the phone.
    env.add_into("box", "wardrobe").success();
    env.add_into("box", "wardrobe").success();
    env.add_into("phone", "wardrobe/box").success();

    // Now one box holds something, so the two are told apart by @id.
    env.add_into("charger", "wardrobe/box")
        .failure()
        .stderr(predicate::str::contains("ambiguous"))
        .stderr(predicate::str::contains("@2"))
        .stderr(predicate::str::contains("@3"));
}

/// Test: duplicate names allowed in different places
#[test]
fn add_duplicate_name_in_different_places_succeeds() {
    let env = common::TestEnv::new();

    // Add hammer at root
    env.add("hammer").success();

    // Add another hammer in toolbox - should succeed (different place)
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
        .stdout(predicate::str::contains("id,name,description,kind,place"))
        .stdout(predicate::str::contains("hammer"));
}

/// Test: a name with '/' is refused, because '/' separates a path
#[test]
fn add_name_with_slash_fails() {
    let env = common::TestEnv::new();

    env.add("usb a/c adapter")
        .failure()
        .stderr(predicate::str::contains("cannot contain '/'"));
}

/// Test: an empty or blank name is refused
#[test]
fn add_blank_name_fails() {
    let env = common::TestEnv::new();

    env.add("")
        .failure()
        .stderr(predicate::str::contains("cannot be empty"));
    env.add("   ")
        .failure()
        .stderr(predicate::str::contains("cannot be empty"));
}

/// Test: surrounding whitespace is trimmed from the name and description
#[test]
fn add_trims_name_and_description() {
    let env = common::TestEnv::new();

    env.add_with_desc("  hammer  ", "   ").success();

    env.run(&["show", "hammer", "--json"])
        .success()
        .stdout(predicate::str::contains(r#""name":"hammer""#))
        .stdout(predicate::str::contains("description").not());
}

/// Test: a refused add leaves no auto-created place behind
#[test]
fn add_refused_does_not_create_the_place() {
    let env = common::TestEnv::new();

    env.add_into("a/b", "shed").failure();

    env.run(&["show", "shed"]).failure();
}

/// Test: `--in /` adds at root, the same as `mv`
#[test]
fn add_into_slash_is_root() {
    let env = common::TestEnv::new();

    env.add_into("hammer", "/").success();

    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("hammer"));
}
