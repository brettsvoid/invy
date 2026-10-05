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
/// * `all` - Move every duplicate the reference matches
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(
    item_ref: &str,
    destination: &str,
    all: bool,
    json: bool,
    csv: bool,
    db_path: Option<&Path>,
) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    let (items, matched) = inventory::select(&conn, item_ref, all)?;

    // Several items are only ever duplicates, which share a place.
    let old_path = db::get_item_path(&conn, items[0].id)?;

    // A refused move must not leave an auto-created place behind.
    let tx = conn.unchecked_transaction()?;
    let new_place_id = inventory::resolve_destination(&tx, destination)?;
    for item in &items {
        inventory::move_to(&tx, item, new_place_id)?;
    }
    tx.commit()?;

    let mut moved = Vec::with_capacity(items.len());
    for item in items {
        let updated = db::get_item_by_id(&conn, item.id)?
            .ok_or_else(|| anyhow!("Failed to retrieve moved item"))?;
        let path = db::get_item_path(&conn, item.id)?;
        moved.push(updated.with_path(path, None));
    }

    output::print_moved(&moved, matched, &old_path, format)
}
