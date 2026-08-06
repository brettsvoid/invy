//! Find command implementation.
//!
//! See SPEC.md#invy-find-query

use anyhow::Result;
use std::path::Path;

use anyhow::anyhow;

use crate::db;
use crate::model::Kind;
use crate::output::{self, Format};

/// Search for items by name or description.
///
/// # Arguments
/// * `query` - Search term (substring match, case-insensitive)
/// * `kind` - Optional kind to filter by
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(
    query: Option<&str>,
    kind: Option<Kind>,
    json: bool,
    csv: bool,
    db_path: Option<&Path>,
) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    let items = match (kind, query) {
        (Some(kind), query) => db::find_items_by_kind(&conn, kind, query)?,
        (None, Some(query)) => db::search_items(&conn, query)?,
        (None, None) => return Err(anyhow!("give a search term, a --kind, or both")),
    };

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
