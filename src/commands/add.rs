//! Add command implementation.
//!
//! See SPEC.md#invy-add-name

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::db;
use crate::output::{self, Format};

/// Add a new item to the inventory.
///
/// # Arguments
/// * `name` - Name of the item
/// * `desc` - Optional description
/// * `place` - Optional place to put the item in (auto-creates if needed)
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(
    name: &str,
    desc: Option<&str>,
    place: Option<&str>,
    json: bool,
    csv: bool,
    db_path: Option<&Path>,
) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    // Resolve place if specified
    let place_id = match place {
        Some(place_ref) => {
            let place_item = db::resolve_or_create_place(&conn, place_ref)?;
            Some(place_item.id)
        }
        None => None,
    };

    // Check for duplicate name in same place
    if db::name_exists_in_place(&conn, name, place_id)? {
        let location = match place {
            Some(c) => c.to_string(),
            None => "(root)".to_string(),
        };
        return Err(anyhow!("item '{}' already exists in {}", name, location));
    }

    // Insert the item
    let item = db::insert_item(&conn, name, desc, place_id)?;

    // Get full path for display
    let path = db::get_item_path(&conn, item.id)?;
    let child_count = db::count_children(&conn, item.id)?;
    let item_with_path = item.with_path(path, Some(child_count));

    output::print_added(&item_with_path, format)
}
