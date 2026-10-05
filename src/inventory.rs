//! The rules every change to the inventory must follow.
//!
//! The CLI commands and the TUI both go through here, so a name, move or
//! reference rule lives in one place. db.rs stays plain SQL.

use anyhow::{anyhow, Result};
use rusqlite::Connection;

use crate::db;
use crate::model::{Item, Kind};

/// Trim a name and check it can be stored.
///
/// A `/` would split the name when it is read back as a path.
fn clean_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("name cannot be empty"));
    }
    if name.contains('/') {
        return Err(anyhow!("name cannot contain '/'"));
    }
    Ok(name)
}

/// Trim a description. A blank one is no description.
fn clean_description(text: &str) -> Option<&str> {
    Some(text.trim()).filter(|text| !text.is_empty())
}

/// Read `@14` as the id 14. Anything else is a name or a path.
fn parse_id(reference: &str) -> Option<i64> {
    let digits = reference.strip_prefix('@')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Find the item a reference means: an `@id`, a path, or a name anywhere.
///
/// When the reference matches duplicates, any one will do, so this returns
/// the oldest. Matches that are not interchangeable are an error.
pub fn resolve(conn: &Connection, reference: &str) -> Result<Option<Item>> {
    Ok(resolve_all(conn, reference)?.into_iter().next())
}

/// Find every item a reference means: one item, or a set of duplicates.
pub fn resolve_all(conn: &Connection, reference: &str) -> Result<Vec<Item>> {
    let reference = reference.trim();
    if let Some(id) = parse_id(reference) {
        return Ok(db::get_item_by_id(conn, id)?.into_iter().collect());
    }

    let matches = if reference.contains('/') {
        db::find_items_by_path(conn, reference)?
    } else {
        db::find_items_by_exact_name(conn, reference)?
    };
    ensure_interchangeable(conn, reference, &matches)?;
    Ok(matches)
}

/// The items a command acts on: the first match, or every match with `all`.
///
/// Also returns how many matched, so output can say "1 of 3".
pub fn select(conn: &Connection, reference: &str, all: bool) -> Result<(Vec<Item>, usize)> {
    let mut matches = resolve_all(conn, reference)?;
    if matches.is_empty() {
        return Err(anyhow!("item '{}' not found", reference.trim()));
    }
    let matched = matches.len();
    if !all {
        matches.truncate(1);
    }
    Ok((matches, matched))
}

/// Turn a destination as typed into a place id, creating the place if needed.
///
/// `/`, `root` and an empty string all mean root. An `@id` must exist: it is
/// never taken as the name of a new place.
pub fn resolve_destination(conn: &Connection, reference: &str) -> Result<Option<i64>> {
    let reference = reference.trim();
    if reference.is_empty() || reference == "/" || reference == "root" {
        return Ok(None);
    }
    if let Some(place) = resolve(conn, reference)? {
        return Ok(Some(place.id));
    }
    if parse_id(reference).is_some() {
        return Err(anyhow!("item '{reference}' not found"));
    }

    // Nothing matched, so walk the path from root, keeping the places that
    // exist and creating the rest. A bare name is created at root.
    let mut place_id = None;
    for part in reference.split('/').filter(|part| !part.trim().is_empty()) {
        let existing = db::find_items_named_in(conn, part.trim(), place_id)?;
        ensure_interchangeable(conn, part.trim(), &existing)?;
        place_id = Some(match existing.into_iter().next() {
            Some(place) => place.id,
            None => db::insert_item(conn, clean_name(part)?, None, place_id, Kind::Thing)?.id,
        });
    }
    Ok(place_id)
}

/// Whether two items are duplicates, leaving aside whether they hold anything.
fn same_apart_from_id(a: &Item, b: &Item) -> bool {
    a.duplicate_key() == b.duplicate_key()
}

/// How many duplicates share the item's place, the item included.
pub fn duplicate_count(conn: &Connection, item: &Item) -> Result<usize> {
    if db::count_children(conn, item.id)? > 0 {
        return Ok(1);
    }
    let mut count = 0;
    for other in db::find_items_named_in(conn, &item.name, item.place_id)? {
        if same_apart_from_id(item, &other) && db::count_children(conn, other.id)? == 0 {
            count += 1;
        }
    }
    Ok(count)
}

/// Refuse a reference that matches items that are not duplicates.
fn ensure_interchangeable(conn: &Connection, reference: &str, matches: &[Item]) -> Result<()> {
    let [first, ..] = matches else {
        return Ok(());
    };
    if matches.len() == 1 {
        return Ok(());
    }

    let mut duplicates = true;
    for item in matches {
        if !same_apart_from_id(first, item) || db::count_children(conn, item.id)? > 0 {
            duplicates = false;
            break;
        }
    }
    if duplicates {
        return Ok(());
    }

    let mut lines = Vec::with_capacity(matches.len());
    for item in matches {
        let path = db::get_item_path(conn, item.id)?.join("/");
        lines.push(match &item.description {
            Some(desc) => format!("  @{}  {path}  {desc}", item.id),
            None => format!("  @{}  {path}", item.id),
        });
    }
    Err(anyhow!(
        "'{reference}' is ambiguous. Use a path or an @id:\n{}",
        lines.join("\n")
    ))
}

/// Add an item to a place, or to root when `place_id` is `None`.
pub fn add(
    conn: &Connection,
    name: &str,
    description: Option<&str>,
    place_id: Option<i64>,
    kind: Kind,
) -> Result<Item> {
    let name = clean_name(name)?;
    let description = description.and_then(clean_description);
    db::insert_item(conn, name, description, place_id, kind)
}

/// Rename an item. Returns false when the new name is the old one.
pub fn rename(conn: &Connection, item: &Item, new_name: &str) -> Result<bool> {
    let new_name = clean_name(new_name)?;
    if new_name == item.name {
        return Ok(false);
    }
    db::update_item_name(conn, item.id, new_name)?;
    Ok(true)
}

/// Set or clear an item's description. Returns true when one is now set.
pub fn describe(conn: &Connection, id: i64, text: &str) -> Result<bool> {
    let description = clean_description(text);
    db::update_item_description(conn, id, description)?;
    Ok(description.is_some())
}

/// Move an item into a place, or to root when `place_id` is `None`.
pub fn move_to(conn: &Connection, item: &Item, place_id: Option<i64>) -> Result<()> {
    if let Some(place_id) = place_id {
        if place_id == item.id || db::is_ancestor(conn, item.id, place_id)? {
            return Err(anyhow!(
                "cannot move '{}' into itself or its descendants",
                item.name
            ));
        }
    }
    db::move_item(conn, item.id, place_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed() {
        assert_eq!(clean_name("  hammer \t").unwrap(), "hammer");
    }

    #[test]
    fn blank_and_slashed_names_are_refused() {
        for name in ["", "   ", "a/b", "/"] {
            assert!(clean_name(name).is_err(), "{name:?} was accepted");
        }
    }

    #[test]
    fn blank_descriptions_clear() {
        assert_eq!(clean_description("  "), None);
        assert_eq!(clean_description(" 16oz "), Some("16oz"));
    }

    #[test]
    fn only_an_at_sign_and_digits_is_an_id() {
        assert_eq!(parse_id("@14"), Some(14));
        for reference in ["@", "14", "@14a", "@ 14", "@-1", "hammer"] {
            assert_eq!(parse_id(reference), None, "{reference:?}");
        }
    }
}
