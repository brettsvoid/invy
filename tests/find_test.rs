//! Integration tests for the `find` command.
//!
//! See SPEC.md#invy-find-query

mod common;

use predicates::prelude::*;

/// Test: find by exact name
#[test]
fn find_by_exact_name() {
    let env = common::TestEnv::new();

    // Setup
    env.add_with_desc("hammer", "claw hammer").success();

    // Find
    env.run(&["find", "hammer"])
        .success()
        .stdout(predicate::str::contains("hammer"));
}

/// Test: find by partial name (substring)
#[test]
fn find_by_partial_name() {
    let env = common::TestEnv::new();

    // Setup
    env.add("screwdriver").success();

    // Find by substring
    env.run(&["find", "screw"])
        .success()
        .stdout(predicate::str::contains("screwdriver"));
}

/// Test: find by description content
#[test]
fn find_by_description() {
    let env = common::TestEnv::new();

    // Setup
    env.add_with_desc("hammer", "phillips head tool").success();

    // Find by description content
    env.run(&["find", "phillips"])
        .success()
        .stdout(predicate::str::contains("hammer"));
}

/// Test: find returns full path to item
#[test]
fn find_returns_full_path() {
    let env = common::TestEnv::new();

    // Setup hierarchy: garage -> toolbox -> hammer
    env.add("garage").success();
    env.add_into("toolbox", "garage").success();
    env.add_into("hammer", "toolbox").success();

    // Find should show the path
    env.run(&["find", "hammer"])
        .success()
        .stdout(predicate::str::contains("toolbox"))
        .stdout(predicate::str::contains("garage"));
}

/// Test: find with no results
#[test]
fn find_with_no_results() {
    let env = common::TestEnv::new();

    // Setup
    env.add("hammer").success();

    // Search for something that doesn't exist
    env.run(&["find", "nonexistent"]).success(); // Should still succeed, just empty output
}

/// Test: find is case-insensitive
#[test]
fn find_is_case_insensitive() {
    let env = common::TestEnv::new();

    // Setup with lowercase
    env.add("hammer").success();

    // Find with uppercase
    env.run(&["find", "HAMMER"])
        .success()
        .stdout(predicate::str::contains("hammer"));
}

/// Test: find with JSON output
#[test]
fn find_with_json_output() {
    let env = common::TestEnv::new();

    // Setup
    env.add_with_desc("hammer", "claw hammer").success();

    // Find with JSON
    env.run(&["find", "hammer", "--json"])
        .success()
        .stdout(predicate::str::contains(r#""name":"hammer""#))
        .stdout(predicate::str::contains("path"));
}

/// Test: find with CSV output
#[test]
fn find_with_csv_output() {
    let env = common::TestEnv::new();

    // Setup
    env.add("hammer").success();

    // Find with CSV
    env.run(&["find", "hammer", "--csv"])
        .success()
        .stdout(predicate::str::contains("id,name,description,path"));
}
