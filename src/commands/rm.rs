//! Remove command implementation.
//!
//! See SPEC.md#invy-rm-item

use anyhow::Result;
use std::path::Path;

use crate::db;
use crate::inventory;
use crate::output::{self, Format};

/// Remove an item from the inventory.
///
/// If the item is a place with children, orphan them to root level.
///
/// # Arguments
/// * `item` - Item to remove
/// * `all` - Remove every duplicate the reference matches
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(item_ref: &str, all: bool, json: bool, csv: bool, db_path: Option<&Path>) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    let (items, matched) = inventory::select(&conn, item_ref, all)?;
    let item_name = items[0].name.clone();

    // Only a single place can hold anything, since duplicates hold nothing.
    let children = db::list_items_in_place(&conn, items[0].id)?;
    let orphaned_names: Vec<String> = children.iter().map(|c| c.name.clone()).collect();

    // The ON DELETE SET NULL will automatically orphan children to root
    // when we delete the place
    let tx = conn.unchecked_transaction()?;
    for item in &items {
        db::delete_item(&tx, item.id)?;
    }
    tx.commit()?;

    output::print_removed(&item_name, items.len(), matched, &orphaned_names, format)
}
