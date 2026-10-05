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
/// * `count` - How many to add, each its own item
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
#[allow(clippy::too_many_arguments)]
pub fn run(
    name: &str,
    desc: Option<&str>,
    place: Option<&str>,
    kind: Kind,
    count: u32,
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
    let mut added = Vec::with_capacity(count as usize);
    for _ in 0..count {
        added.push(inventory::add(&tx, name, desc, place_id, kind)?);
    }
    tx.commit()?;

    // Get full paths for display
    let mut items = Vec::with_capacity(added.len());
    for item in added {
        let path = db::get_item_path(&conn, item.id)?;
        items.push(item.with_path(path, Some(0)));
    }

    output::print_added(&items, format)
}
