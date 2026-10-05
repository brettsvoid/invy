//! Move command implementation.
//!
//! See SPEC.md#invy-mv-item-destination

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::db;
use crate::inventory;
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
    let item = inventory::resolve(&conn, item_ref)?
        .ok_or_else(|| anyhow!("item '{}' not found", item_ref))?;

    // Get old path for display
    let old_path = db::get_item_path(&conn, item.id)?;

    // A refused move must not leave an auto-created place behind.
    let tx = conn.unchecked_transaction()?;
    let new_place_id = inventory::resolve_destination(&tx, destination)?;
    inventory::move_to(&tx, &item, new_place_id)?;
    tx.commit()?;

    // Get updated item for display
    let updated_item = db::get_item_by_id(&conn, item.id)?
        .ok_or_else(|| anyhow!("Failed to retrieve moved item"))?;
    let new_path = db::get_item_path(&conn, updated_item.id)?;
    let item_with_path = updated_item.with_path(new_path, None);

    output::print_moved(&item_with_path, &old_path, format)
}
