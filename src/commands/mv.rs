//! Move command implementation.
//!
//! See SPEC.md#invy-mv-item-destination

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::db;
use crate::output::{self, Format};

/// Move an item to a different place.
///
/// # Arguments
/// * `item` - Item to move
/// * `destination` - Target place (use "/" for root)
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(
    item_ref: &str,
    destination: &str,
    json: bool,
    csv: bool,
    db_path: Option<&Path>,
) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    // Resolve the item to move
    let item = db::resolve_item(&conn, item_ref)?
        .ok_or_else(|| anyhow!("item '{}' not found", item_ref))?;

    // Get old path for display
    let old_path = db::get_item_path(&conn, item.id)?;

    // Resolve destination
    let new_place_id = if destination == "/" || destination == "root" {
        None
    } else {
        let place = db::resolve_or_create_place(&conn, destination)?;

        // Check for circular reference
        if place.id == item.id {
            return Err(anyhow!(
                "cannot move '{}' into itself or its descendants",
                item.name
            ));
        }
        if db::is_ancestor(&conn, item.id, place.id)? {
            return Err(anyhow!(
                "cannot move '{}' into itself or its descendants",
                item.name
            ));
        }

        Some(place.id)
    };

    // Check for name conflict in destination
    if db::name_exists_in_place(&conn, &item.name, new_place_id)? {
        // Check if it's the same item (moving to same place)
        if item.place_id != new_place_id {
            let dest_name = if destination == "/" || destination == "root" {
                "(root)".to_string()
            } else {
                destination.to_string()
            };
            return Err(anyhow!(
                "item '{}' already exists in {}",
                item.name,
                dest_name
            ));
        }
    }

    // Perform the move
    db::move_item(&conn, item.id, new_place_id)?;

    // Get updated item for display
    let updated_item = db::get_item_by_id(&conn, item.id)?
        .ok_or_else(|| anyhow!("Failed to retrieve moved item"))?;
    let new_path = db::get_item_path(&conn, updated_item.id)?;
    let item_with_path = updated_item.with_path(new_path, None);

    output::print_moved(&item_with_path, &old_path, format)
}
