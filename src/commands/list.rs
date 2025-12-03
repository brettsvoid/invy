//! List command implementation.
//!
//! See SPEC.md#invy-list-container

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::db;
use crate::output::{self, Format};

/// List items, optionally within a specific container.
///
/// # Arguments
/// * `container` - Optional container to list (default: root)
/// * `recursive` - List all descendants
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `quiet` - Minimal output
/// * `db_path` - Optional custom database path
pub fn run(
    container: Option<&str>,
    recursive: bool,
    json: bool,
    csv: bool,
    quiet: bool,
    db_path: Option<&Path>,
) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    let items = if recursive {
        // List all items
        db::list_all_items(&conn)?
    } else if let Some(container_ref) = container {
        // List items in specific container
        let container_item = db::resolve_item(&conn, container_ref)?
            .ok_or_else(|| anyhow!("container '{}' not found", container_ref))?;
        db::list_items_in_container(&conn, container_item.id)?
    } else {
        // List root items
        db::list_root_items(&conn)?
    };

    if quiet {
        for item in &items {
            println!("{}", item.id);
        }
        return Ok(());
    }

    // Convert to ListItem with child counts
    let list_items: Vec<_> = items
        .into_iter()
        .map(|item| {
            let child_count = db::count_children(&conn, item.id).unwrap_or(0);
            item.into_list_item(child_count)
        })
        .collect();

    output::print_list_items(&list_items, format)
}
