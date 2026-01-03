//! List command implementation.
//!
//! See SPEC.md#invy-list-container

use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;

use crate::db;
use crate::model::{Item, TreeItem};
use crate::output::{self, Format};

/// List items, optionally within a specific container.
///
/// # Arguments
/// * `container` - Optional container to list (default: root)
/// * `recursive` - List all descendants
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(
    container: Option<&str>,
    recursive: bool,
    json: bool,
    csv: bool,
    db_path: Option<&Path>,
) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    if recursive {
        // Build tree structure for recursive listing
        let items = db::list_all_items(&conn)?;
        let tree = build_item_tree(&items, &conn);
        output::print_tree_items(&tree, format)
    } else {
        let items = if let Some(container_ref) = container {
            // List items in specific container
            let container_item = db::resolve_item(&conn, container_ref)?
                .ok_or_else(|| anyhow!("container '{}' not found", container_ref))?;
            db::list_items_in_container(&conn, container_item.id)?
        } else {
            // List root items
            db::list_root_items(&conn)?
        };

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
}

/// Build a tree structure from flat items using container_id relationships.
fn build_item_tree(items: &[Item], conn: &Connection) -> Vec<TreeItem> {
    // Build parent -> children mapping
    let mut children_map: HashMap<Option<i64>, Vec<&Item>> = HashMap::new();
    for item in items {
        children_map
            .entry(item.container_id)
            .or_default()
            .push(item);
    }

    // Sort children alphabetically (case-insensitive)
    for children in children_map.values_mut() {
        children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    // Recursive tree builder
    fn build_subtree(
        parent_id: Option<i64>,
        children_map: &HashMap<Option<i64>, Vec<&Item>>,
        conn: &Connection,
    ) -> Vec<TreeItem> {
        children_map
            .get(&parent_id)
            .map(|children| {
                children
                    .iter()
                    .map(|item| {
                        let child_count = db::count_children(conn, item.id).unwrap_or(0);
                        TreeItem {
                            id: item.id,
                            name: item.name.clone(),
                            description: item.description.clone(),
                            child_count,
                            children: build_subtree(Some(item.id), children_map, conn),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    build_subtree(None, &children_map, conn)
}
