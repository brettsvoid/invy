//! Find command implementation.
//!
//! See SPEC.md#invy-find-query

use anyhow::Result;
use std::path::Path;

use crate::db;
use crate::output::{self, Format};

/// Search for items by name or description.
///
/// # Arguments
/// * `query` - Search term (substring match, case-insensitive)
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `quiet` - Minimal output
/// * `db_path` - Optional custom database path
pub fn run(query: &str, json: bool, csv: bool, quiet: bool, db_path: Option<&Path>) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    let items = db::search_items(&conn, query)?;

    if quiet {
        for item in &items {
            println!("{}", item.id);
        }
        return Ok(());
    }

    // Convert to ItemWithPath for display
    let items_with_path: Vec<_> = items
        .into_iter()
        .map(|item| {
            let path = db::get_item_path(&conn, item.id).unwrap_or_default();
            item.with_path(path, None)
        })
        .collect();

    output::print_items(&items_with_path, format)
}
