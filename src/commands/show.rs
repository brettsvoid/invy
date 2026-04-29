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

    let item = match db::resolve_item(&conn, item_ref)? {
        Some(item) => item,
        None => {
            let suggestions = db::search_items(&conn, item_ref).unwrap_or_default();
            if !suggestions.is_empty() {
                eprintln!("Did you mean:");
                for suggestion in suggestions.iter().take(10) {
                    let path = db::get_item_path(&conn, suggestion.id).unwrap_or_default();
                    eprintln!("  {}", path.join("/"));
                }
            }
            return Err(anyhow!("item '{}' not found", item_ref));
        }
    };

    let path = db::get_item_path(&conn, item.id)?;
    let child_count = db::count_children(&conn, item.id)?;
    let item_with_path = item.with_path(path, Some(child_count));

    output::print_item(&item_with_path, format)
}
