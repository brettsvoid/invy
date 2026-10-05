//! Add command implementation.
//!
//! See SPEC.md#invy-add-name

use anyhow::Result;
use std::path::Path;

use crate::db;
use crate::inventory;
use crate::model::Kind;
use crate::output::{self, Format};

/// Add a new item to the inventory.
///
/// # Arguments
/// * `name` - Name of the item
/// * `desc` - Optional description
/// * `place` - Optional place to put the item in (auto-creates if needed)
/// * `kind` - What sort of thing this is
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(
    name: &str,
    desc: Option<&str>,
    place: Option<&str>,
    kind: Kind,
    json: bool,
    csv: bool,
    db_path: Option<&Path>,
) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    // A refused add must not leave an auto-created place behind.
    let tx = conn.unchecked_transaction()?;
    let place_id = match place {
        Some(place_ref) => inventory::resolve_destination(&tx, place_ref)?,
        None => None,
    };
    let item = inventory::add(&tx, name, desc, place_id, kind)?;
    tx.commit()?;

    // Get full path for display
    let path = db::get_item_path(&conn, item.id)?;
    let child_count = db::count_children(&conn, item.id)?;
    let item_with_path = item.with_path(path, Some(child_count));

    output::print_added(&item_with_path, format)
}
