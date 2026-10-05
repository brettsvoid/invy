//! Find command implementation.
//!
//! See SPEC.md#invy-find-query

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::db;
use crate::model::Kind;
use crate::output::{self, Format};
use crate::search;

/// Search for items by name or description.
///
/// # Arguments
/// * `query` - Search in fzf's syntax. See the search module
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

    if kind.is_none() && query.is_none() {
        return Err(anyhow!("give a search term, a --kind, or both"));
    }

    let all = db::list_all_items(&conn)?;
    let of_kind = all
        .iter()
        .filter(|item| kind.is_none_or(|kind| item.kind == kind));
    let items = search::search(of_kind, query.unwrap_or(""), |item| {
        db::get_item_path(&conn, item.id)
            .unwrap_or_default()
            .join("/")
            .to_lowercase()
    });

    // Convert to ItemWithPath for display
    let items_with_path: Vec<_> = items
        .into_iter()
        .cloned()
        .map(|item| {
            let path = db::get_item_path(&conn, item.id).unwrap_or_default();
            // Human output needs the count to tell duplicates from places.
            let child_count = db::count_children(&conn, item.id).unwrap_or(0);
            item.with_path(path, Some(child_count))
        })
        .collect();

    output::print_items(&items_with_path, format)
}
