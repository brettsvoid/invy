//! Show command implementation.
//!
//! See SPEC.md#invy-show-item

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::db;
use crate::output::{self, Format};

/// Show detailed information about a specific item.
///
/// # Arguments
/// * `item` - Item name or path
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(item_ref: &str, json: bool, csv: bool, db_path: Option<&Path>) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    let item = db::resolve_item(&conn, item_ref)?
        .ok_or_else(|| anyhow!("item '{}' not found", item_ref))?;

    let path = db::get_item_path(&conn, item.id)?;
    let child_count = db::count_children(&conn, item.id)?;
    let item_with_path = item.with_path(path, Some(child_count));

    output::print_item(&item_with_path, format)
}
