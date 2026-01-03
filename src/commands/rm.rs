//! Remove command implementation.
//!
//! See SPEC.md#invy-rm-item

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::db;
use crate::output::{self, Format};

/// Remove an item from the inventory.
///
/// If the item is a container with children, orphan them to root level.
///
/// # Arguments
/// * `item` - Item to remove
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(item_ref: &str, json: bool, csv: bool, db_path: Option<&Path>) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    // Resolve the item to remove
    let item = db::resolve_item(&conn, item_ref)?
        .ok_or_else(|| anyhow!("item '{}' not found", item_ref))?;

    let item_name = item.name.clone();

    // Get children that will be orphaned
    let children = db::list_items_in_container(&conn, item.id)?;
    let orphaned_names: Vec<String> = children.iter().map(|c| c.name.clone()).collect();

    // The ON DELETE SET NULL will automatically orphan children to root
    // when we delete the container
    db::delete_item(&conn, item.id)?;

    output::print_removed(&item_name, &orphaned_names, format)
}
