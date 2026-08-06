//! Edit command implementation.
//!
//! See SPEC.md#invy-edit-item

use anyhow::{anyhow, Result};
use std::path::Path;

use crate::db;
use crate::model::Kind;
use crate::output::{self, Format};

/// Edit an existing item's name or description.
///
/// # Arguments
/// * `item` - Item to edit
/// * `name` - Optional new name
/// * `desc` - Optional new description (use "" to clear)
/// * `kind` - Optional new kind
/// * `json` - Output as JSON
/// * `csv` - Output as CSV
/// * `db_path` - Optional custom database path
pub fn run(
    item_ref: &str,
    new_name: Option<&str>,
    new_desc: Option<&str>,
    new_kind: Option<Kind>,
    json: bool,
    csv: bool,
    db_path: Option<&Path>,
) -> Result<()> {
    let conn = db::open(db_path)?;
    let format = Format::from_flags(json, csv);

    // Check that at least one change is specified
    if new_name.is_none() && new_desc.is_none() && new_kind.is_none() {
        return Err(anyhow!(
            "no changes specified. Use --name, --desc or --kind"
        ));
    }

    // Resolve the item to edit
    let item = db::resolve_item(&conn, item_ref)?
        .ok_or_else(|| anyhow!("item '{}' not found", item_ref))?;

    let old_name = item.name.clone();
    let old_desc = item.description.clone();
    let old_kind = item.kind;

    // Update name if specified
    if let Some(name) = new_name {
        // Check for name conflict
        if name != item.name && db::name_exists_in_place(&conn, name, item.place_id)? {
            let location = if item.place_id.is_some() {
                "place"
            } else {
                "(root)"
            };
            return Err(anyhow!("item '{}' already exists in {}", name, location));
        }
        db::update_item_name(&conn, item.id, name)?;
    }

    // Update description if specified
    if let Some(desc) = new_desc {
        let desc_value = if desc.is_empty() { None } else { Some(desc) };
        db::update_item_description(&conn, item.id, desc_value)?;
    }

    // Update kind if specified
    if let Some(kind) = new_kind {
        db::update_item_kind(&conn, item.id, kind)?;
    }

    // Get updated item for display
    let updated_item = db::get_item_by_id(&conn, item.id)?
        .ok_or_else(|| anyhow!("Failed to retrieve updated item"))?;
    let path = db::get_item_path(&conn, updated_item.id)?;
    let item_with_path = updated_item.with_path(path, None);

    output::print_updated(
        &item_with_path,
        if new_name.is_some() {
            Some(&old_name)
        } else {
            None
        },
        if new_desc.is_some() {
            Some(old_desc.as_deref())
        } else {
            None
        },
        if new_kind.is_some() {
            Some(old_kind)
        } else {
            None
        },
        format,
    )
}
