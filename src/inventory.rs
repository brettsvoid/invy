//! The rules every change to the inventory must follow.
//!
//! The CLI commands and the TUI both go through here, so a name or move rule
//! lives in one place. db.rs stays plain SQL.

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

/// Turn a destination as typed into a place id, creating the place if needed.
///
/// `/`, `root` and an empty string all mean root.
pub fn resolve_destination(conn: &Connection, reference: &str) -> Result<Option<i64>> {
    let reference = reference.trim();
    if reference.is_empty() || reference == "/" || reference == "root" {
        return Ok(None);
    }
    Ok(Some(db::resolve_or_create_place(conn, reference)?.id))
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
    ensure_name_free(conn, name, place_id)?;
    let description = description.and_then(clean_description);
    db::insert_item(conn, name, description, place_id, kind)
}

/// Rename an item. Returns false when the new name is the old one.
pub fn rename(conn: &Connection, item: &Item, new_name: &str) -> Result<bool> {
    let new_name = clean_name(new_name)?;
    if new_name == item.name {
        return Ok(false);
    }
    ensure_name_free(conn, new_name, item.place_id)?;
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
    if item.place_id != place_id {
        ensure_name_free(conn, &item.name, place_id)?;
    }
    db::move_item(conn, item.id, place_id)
}

fn ensure_name_free(conn: &Connection, name: &str, place_id: Option<i64>) -> Result<()> {
    if !db::name_exists_in_place(conn, name, place_id)? {
        return Ok(());
    }
    let place = match place_id {
        Some(id) => db::get_item_path(conn, id)?.join("/"),
        None => "(root)".to_string(),
    };
    Err(anyhow!("item '{name}' already exists in {place}"))
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
}
