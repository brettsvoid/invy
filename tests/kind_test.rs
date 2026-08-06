//! Tests for item kinds.
//!
//! See SPEC.md#kinds

mod common;

use common::TestEnv;
use predicates::prelude::*;

#[test]
fn an_item_added_without_a_kind_is_a_thing() {
    let env = TestEnv::new();
    env.add("hammer").success();

    env.run(&["show", "hammer"])
        .success()
        .stdout(predicate::str::contains("Kind:        thing"));
}

#[test]
fn add_records_the_given_kind() {
    let env = TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();

    env.run(&["show", "garage"])
        .success()
        .stdout(predicate::str::contains("Kind:        room"));
}

#[test]
fn add_rejects_a_kind_that_does_not_exist() {
    let env = TestEnv::new();
    env.run(&["add", "garage", "--kind", "bocks"])
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn edit_changes_the_kind() {
    let env = TestEnv::new();
    env.add("garage").success();

    env.run(&["edit", "garage", "--kind", "room"]).success();
    env.run(&["show", "garage"])
        .success()
        .stdout(predicate::str::contains("Kind:        room"));

    // And back again, so a mistake is reversible without a special flag.
    env.run(&["edit", "garage", "--kind", "thing"]).success();
    env.run(&["show", "garage"])
        .success()
        .stdout(predicate::str::contains("Kind:        thing"));
}

#[test]
fn edit_with_no_arguments_names_kind_as_an_option() {
    let env = TestEnv::new();
    env.add("hammer").success();

    env.run(&["edit", "hammer"])
        .failure()
        .stderr(predicate::str::contains("--name, --desc or --kind"));
}

#[test]
fn find_by_kind_alone_needs_no_search_term() {
    let env = TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();
    env.run(&["add", "attic", "--kind", "room"]).success();
    env.run(&["add", "toolbox", "--in", "garage", "--kind", "box"])
        .success();
    env.add_into("hammer", "garage/toolbox").success();

    env.run(&["find", "--kind", "room"])
        .success()
        .stdout(predicate::str::contains("garage"))
        .stdout(predicate::str::contains("attic"))
        .stdout(predicate::str::contains("hammer").not())
        .stdout(predicate::str::contains("toolbox").not());
}

#[test]
fn find_combines_a_kind_with_a_search_term() {
    let env = TestEnv::new();
    env.run(&["add", "bedroom", "--kind", "room"]).success();
    env.run(&["add", "bathroom", "--kind", "room"]).success();
    env.run(&["add", "bed", "--in", "bedroom", "--kind", "furniture"])
        .success();

    env.run(&["find", "bed", "--kind", "room"])
        .success()
        .stdout(predicate::str::contains("bedroom"))
        .stdout(predicate::str::contains("bathroom").not());
}

#[test]
fn find_with_neither_a_term_nor_a_kind_is_an_error() {
    let env = TestEnv::new();
    env.add("hammer").success();

    env.run(&["find"])
        .failure()
        .stderr(predicate::str::contains("give a search term"));
}

#[test]
fn list_shows_a_kind_column() {
    let env = TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();

    env.run(&["list"])
        .success()
        .stdout(predicate::str::contains("KIND"))
        .stdout(predicate::str::contains("room"));
}

/// Seed a room holding a box holding a plain thing.
fn seeded() -> TestEnv {
    let env = TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();
    env.run(&["add", "toolbox", "--in", "garage", "--kind", "box"])
        .success();
    env.add_into("hammer", "garage/toolbox").success();
    env
}

#[test]
fn the_recursive_tree_marks_places_with_nerd_glyphs_by_default() {
    let env = seeded();

    env.run(&["list", "--recursive"])
        .success()
        .stdout(predicate::str::contains("\u{f02de} garage"))
        .stdout(predicate::str::contains("\u{f03d7} toolbox"))
        // A plain thing carries no glyph.
        .stdout(predicate::str::contains("── hammer"));
}

#[test]
fn the_unicode_glyph_set_needs_no_patched_font() {
    let env = seeded();

    env.run(&["list", "--recursive", "--glyphs", "unicode"])
        .success()
        .stdout(predicate::str::contains("⌂ garage"))
        .stdout(predicate::str::contains("▣ toolbox"));
}

#[test]
fn the_ascii_glyph_set_draws_no_icons() {
    let env = seeded();

    env.run(&["list", "--recursive", "--glyphs", "ascii"])
        .success()
        .stdout(predicate::str::contains("garage"))
        .stdout(predicate::str::contains("⌂").not())
        .stdout(predicate::str::contains("\u{f02de}").not());
}

#[test]
fn the_glyph_set_can_come_from_the_environment() {
    let env = seeded();

    env.cmd()
        .env("INVY_GLYPHS", "unicode")
        .args(["list", "--recursive"])
        .assert()
        .success()
        .stdout(predicate::str::contains("⌂ garage"));
}

#[test]
fn the_flag_beats_the_environment() {
    let env = seeded();

    env.cmd()
        .env("INVY_GLYPHS", "unicode")
        .args(["list", "--recursive", "--glyphs", "ascii"])
        .assert()
        .success()
        .stdout(predicate::str::contains("⌂").not());
}

#[test]
fn an_unknown_glyph_set_is_rejected() {
    let env = TestEnv::new();

    env.run(&["list", "--glyphs", "emoji"])
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn json_output_carries_the_kind() {
    let env = TestEnv::new();
    env.run(&["add", "garage", "--kind", "room"]).success();

    env.run(&["show", "garage", "--json"])
        .success()
        .stdout(predicate::str::contains(r#""kind":"room""#));
}

#[test]
fn a_place_created_on_the_way_to_an_item_starts_as_a_thing() {
    let env = TestEnv::new();
    // 'loft' is auto-created here, and invy cannot know what sort of place it is.
    env.add_into("boxes", "loft").success();

    env.run(&["show", "loft"])
        .success()
        .stdout(predicate::str::contains("Kind:        thing"));
}

#[test]
fn edit_reports_the_kind_change() {
    let env = TestEnv::new();
    env.add("garage").success();

    env.run(&["edit", "garage", "--kind", "room"])
        .success()
        .stdout(predicate::str::contains("kind: thing -> room"));
}
