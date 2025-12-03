//! Database layer for invy.
//!
//! Provides SQLite connection management, migrations, and CRUD operations.

use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

use crate::model::Item;

/// Get the default database path (~/.invy.db)
pub fn default_db_path() -> Result<PathBuf> {
    let home = directories::BaseDirs::new()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home.home_dir().join(".invy.db"))
}

/// Open a database connection, creating and migrating if necessary.
pub fn open(path: Option<&Path>) -> Result<Connection> {
    let db_path = match path {
        Some(p) => p.to_path_buf(),
        None => default_db_path()?,
    };

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database at {:?}", db_path))?;

    migrate(&conn)?;
    Ok(conn)
}

/// Run database migrations.
fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            container_id INTEGER REFERENCES items(id) ON DELETE SET NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_items_name ON items(name);
        CREATE INDEX IF NOT EXISTS idx_items_container ON items(container_id);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_items_name_container
            ON items(name, COALESCE(container_id, 0));
        "#,
    )
    .context("Failed to run migrations")?;

    Ok(())
}

/// Insert a new item into the database.
pub fn insert_item(
    conn: &Connection,
    name: &str,
    description: Option<&str>,
    container_id: Option<i64>,
) -> Result<Item> {
    conn.execute(
        "INSERT INTO items (name, description, container_id) VALUES (?1, ?2, ?3)",
        params![name, description, container_id],
    )
    .with_context(|| format!("Failed to insert item '{}'", name))?;

    let id = conn.last_insert_rowid();
    get_item_by_id(conn, id)?.ok_or_else(|| anyhow!("Failed to retrieve inserted item"))
}

/// Get an item by ID.
pub fn get_item_by_id(conn: &Connection, id: i64) -> Result<Option<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, container_id, created_at, updated_at FROM items WHERE id = ?1",
    )?;

    let item = stmt
        .query_row(params![id], |row| {
            Ok(Item {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                container_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .optional()?;

    Ok(item)
}

/// Get an item by name. Returns error if ambiguous (multiple matches).
pub fn get_item_by_name(conn: &Connection, name: &str) -> Result<Option<Item>> {
    let items = find_items_by_exact_name(conn, name)?;

    match items.len() {
        0 => Ok(None),
        1 => Ok(Some(items.into_iter().next().unwrap())),
        _ => {
            let paths: Vec<String> = items
                .iter()
                .map(|i| get_item_path(conn, i.id).unwrap_or_default().join("/"))
                .collect();
            Err(anyhow!(
                "'{}' is ambiguous. Use full path: {}",
                name,
                paths.join(", ")
            ))
        }
    }
}

/// Find items by exact name (may return multiple if in different containers).
pub fn find_items_by_exact_name(conn: &Connection, name: &str) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, container_id, created_at, updated_at
         FROM items WHERE name = ?1",
    )?;

    let items = stmt
        .query_map(params![name], |row| {
            Ok(Item {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                container_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// Get item by path (e.g., "garage/toolbox/hammer").
pub fn get_item_by_path(conn: &Connection, path: &str) -> Result<Option<Item>> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if parts.is_empty() {
        return Ok(None);
    }

    let mut current_container_id: Option<i64> = None;
    let mut current_item: Option<Item> = None;

    for part in parts {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, container_id, created_at, updated_at
             FROM items WHERE name = ?1 AND container_id IS ?2",
        )?;

        current_item = stmt
            .query_row(params![part, current_container_id], |row| {
                Ok(Item {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    container_id: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .optional()?;

        match &current_item {
            Some(item) => current_container_id = Some(item.id),
            None => return Ok(None),
        }
    }

    Ok(current_item)
}

/// Resolve an item reference (either name or path).
pub fn resolve_item(conn: &Connection, reference: &str) -> Result<Option<Item>> {
    if reference.contains('/') {
        get_item_by_path(conn, reference)
    } else {
        get_item_by_name(conn, reference)
    }
}

/// Get the path to an item as a vector of names (from root to item).
pub fn get_item_path(conn: &Connection, item_id: i64) -> Result<Vec<String>> {
    let mut path = Vec::new();
    let mut current_id = Some(item_id);

    while let Some(id) = current_id {
        if let Some(item) = get_item_by_id(conn, id)? {
            path.push(item.name);
            current_id = item.container_id;
        } else {
            break;
        }
    }

    path.reverse();
    Ok(path)
}

/// Search items by name or description (case-insensitive substring match).
pub fn search_items(conn: &Connection, query: &str) -> Result<Vec<Item>> {
    let pattern = format!("%{}%", query);

    let mut stmt = conn.prepare(
        "SELECT id, name, description, container_id, created_at, updated_at
         FROM items
         WHERE name LIKE ?1 COLLATE NOCASE
            OR description LIKE ?1 COLLATE NOCASE",
    )?;

    let items = stmt
        .query_map(params![pattern], |row| {
            Ok(Item {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                container_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// List items at root level (no container).
pub fn list_root_items(conn: &Connection) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, container_id, created_at, updated_at
         FROM items WHERE container_id IS NULL",
    )?;

    let items = stmt
        .query_map([], |row| {
            Ok(Item {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                container_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// List items in a specific container.
pub fn list_items_in_container(conn: &Connection, container_id: i64) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, container_id, created_at, updated_at
         FROM items WHERE container_id = ?1",
    )?;

    let items = stmt
        .query_map(params![container_id], |row| {
            Ok(Item {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                container_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// List all items recursively.
pub fn list_all_items(conn: &Connection) -> Result<Vec<Item>> {
    let mut stmt = conn
        .prepare("SELECT id, name, description, container_id, created_at, updated_at FROM items")?;

    let items = stmt
        .query_map([], |row| {
            Ok(Item {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                container_id: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// Count children of an item.
pub fn count_children(conn: &Connection, item_id: i64) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM items WHERE container_id = ?1",
        params![item_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Update an item's name.
pub fn update_item_name(conn: &Connection, item_id: i64, new_name: &str) -> Result<()> {
    conn.execute(
        "UPDATE items SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![new_name, item_id],
    )?;
    Ok(())
}

/// Update an item's description.
pub fn update_item_description(
    conn: &Connection,
    item_id: i64,
    new_description: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE items SET description = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![new_description, item_id],
    )?;
    Ok(())
}

/// Move an item to a new container.
pub fn move_item(conn: &Connection, item_id: i64, new_container_id: Option<i64>) -> Result<()> {
    conn.execute(
        "UPDATE items SET container_id = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![new_container_id, item_id],
    )?;
    Ok(())
}

/// Delete an item by ID.
pub fn delete_item(conn: &Connection, item_id: i64) -> Result<()> {
    conn.execute("DELETE FROM items WHERE id = ?1", params![item_id])?;
    Ok(())
}

/// Check if an item is an ancestor of another item.
pub fn is_ancestor(conn: &Connection, potential_ancestor_id: i64, item_id: i64) -> Result<bool> {
    let mut current_id = Some(item_id);

    while let Some(id) = current_id {
        if id == potential_ancestor_id {
            return Ok(true);
        }
        if let Some(item) = get_item_by_id(conn, id)? {
            current_id = item.container_id;
        } else {
            break;
        }
    }

    Ok(false)
}

/// Check if a name exists in a container.
pub fn name_exists_in_container(
    conn: &Connection,
    name: &str,
    container_id: Option<i64>,
) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM items WHERE name = ?1 AND container_id IS ?2",
        params![name, container_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Get or create a container by name (at root level).
#[allow(dead_code)]
pub fn get_or_create_container(conn: &Connection, name: &str) -> Result<Item> {
    // First try to find existing
    if let Some(item) = get_item_by_path(conn, name)? {
        return Ok(item);
    }

    // Create new container
    insert_item(conn, name, None, None)
}

/// Resolve a container reference, creating if necessary.
pub fn resolve_or_create_container(conn: &Connection, reference: &str) -> Result<Item> {
    // First try to resolve existing
    if let Ok(Some(item)) = resolve_item(conn, reference) {
        return Ok(item);
    }

    // If it's a path, we need to create the hierarchy
    if reference.contains('/') {
        let parts: Vec<&str> = reference.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_container_id: Option<i64> = None;
        let mut current_item: Option<Item> = None;

        for part in parts {
            // Check if this part exists in current container
            let existing = if let Some(cid) = current_container_id {
                let items = list_items_in_container(conn, cid)?;
                items.into_iter().find(|i| i.name == part)
            } else {
                let items = list_root_items(conn)?;
                items.into_iter().find(|i| i.name == part)
            };

            current_item = match existing {
                Some(item) => {
                    current_container_id = Some(item.id);
                    Some(item)
                }
                None => {
                    let new_item = insert_item(conn, part, None, current_container_id)?;
                    current_container_id = Some(new_item.id);
                    Some(new_item)
                }
            };
        }

        current_item.ok_or_else(|| anyhow!("Failed to create container path"))
    } else {
        // Simple name - create at root
        insert_item(conn, reference, None, None)
    }
}
