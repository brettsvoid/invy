//! Database layer for invy.
//!
//! Provides SQLite connection management, migrations, and CRUD operations.

use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

use crate::config;
use crate::model::{Item, Kind};

/// Get the default database path (~/.local/share/invy/invy.db)
pub fn default_db_path() -> Result<PathBuf> {
    Ok(config::data_dir()?.join("invy.db"))
}

/// Open a database connection, creating and migrating if necessary.
///
/// The default location's directory is created if missing. A path the user
/// gave is not: its directory has to exist already.
pub fn open(path: Option<&Path>) -> Result<Connection> {
    let db_path = match path {
        Some(p) => p.to_path_buf(),
        None => {
            let path = default_db_path()?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)
                    .with_context(|| format!("Failed to create {:?}", dir))?;
            }
            path
        }
    };

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database at {:?}", db_path))?;

    migrate(&conn)?;
    Ok(conn)
}

/// Schema version this build expects. Bump it for every new migration step.
const SCHEMA_VERSION: i64 = 4;

/// Bring the database up to `SCHEMA_VERSION`.
///
/// `PRAGMA user_version` records how far a file has come. Version 1 is the
/// original schema, which named the parent column `container_id`.
fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if version >= SCHEMA_VERSION {
        return Ok(());
    }

    if version < 1 {
        // Either a new file, or one written before user_version was set. The
        // IF NOT EXISTS guards make the second case a no-op.
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
        .context("Failed to create the items table")?;
    }

    if version < 2 {
        // Rename container_id to place_id. SQLite rewrites the foreign key and
        // the index expressions, but keeps the old index names, so drop and
        // recreate those.
        if has_column(conn, "items", "container_id")? {
            conn.execute_batch("ALTER TABLE items RENAME COLUMN container_id TO place_id")
                .context("Failed to rename container_id to place_id")?;
        }
        conn.execute_batch(
            r#"
            DROP INDEX IF EXISTS idx_items_container;
            DROP INDEX IF EXISTS idx_items_name_container;
            CREATE INDEX IF NOT EXISTS idx_items_place ON items(place_id);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_items_name_place
                ON items(name, COALESCE(place_id, 0));
            "#,
        )
        .context("Failed to rebuild the place indexes")?;
    }

    if version < 3 {
        // Every existing row becomes a 'thing'. The user reclassifies the
        // places afterwards, with `invy edit --kind` or the TUI.
        if !has_column(conn, "items", "kind")? {
            conn.execute_batch("ALTER TABLE items ADD COLUMN kind TEXT NOT NULL DEFAULT 'thing'")
                .context("Failed to add the kind column")?;
        }
        conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_items_kind ON items(kind)")
            .context("Failed to create the kind index")?;
    }

    if version < 4 {
        // Identical things are separate items, so a place can hold the same
        // name twice. See docs/adr/0001-no-item-quantities.md.
        conn.execute_batch("DROP INDEX IF EXISTS idx_items_name_place")
            .context("Failed to drop the unique name index")?;
    }

    conn.pragma_update(None, "user_version", SCHEMA_VERSION)
        .context("Failed to record the schema version")?;

    Ok(())
}

/// Build an `Item` from a row selecting the columns in their canonical order.
fn item_from_row(row: &rusqlite::Row) -> rusqlite::Result<Item> {
    Ok(Item {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        place_id: row.get(3)?,
        kind: Kind::from_db(&row.get::<_, String>(4)?),
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

/// Whether `table` currently has a column called `column`.
fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Insert a new item into the database.
pub fn insert_item(
    conn: &Connection,
    name: &str,
    description: Option<&str>,
    place_id: Option<i64>,
    kind: Kind,
) -> Result<Item> {
    conn.execute(
        "INSERT INTO items (name, description, place_id, kind) VALUES (?1, ?2, ?3, ?4)",
        params![name, description, place_id, kind],
    )
    .with_context(|| format!("Failed to insert item '{}'", name))?;

    let id = conn.last_insert_rowid();
    get_item_by_id(conn, id)?.ok_or_else(|| anyhow!("Failed to retrieve inserted item"))
}

/// Get an item by ID.
pub fn get_item_by_id(conn: &Connection, id: i64) -> Result<Option<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, place_id, kind, created_at, updated_at FROM items WHERE id = ?1",
    )?;

    let item = stmt.query_row(params![id], item_from_row).optional()?;

    Ok(item)
}

/// Find items by name anywhere in the tree, ignoring case.
pub fn find_items_by_exact_name(conn: &Connection, name: &str) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, place_id, kind, created_at, updated_at
         FROM items WHERE name = ?1 COLLATE NOCASE ORDER BY id",
    )?;

    let items = stmt
        .query_map(params![name], item_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// Find items by name directly inside one place, or at root, ignoring case.
pub fn find_items_named_in(
    conn: &Connection,
    name: &str,
    place_id: Option<i64>,
) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, place_id, kind, created_at, updated_at
         FROM items WHERE name = ?1 COLLATE NOCASE AND place_id IS ?2 ORDER BY id",
    )?;

    let items = stmt
        .query_map(params![name, place_id], item_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// Find every item at a path (e.g., "garage/toolbox/hammer").
///
/// Names repeat, so each step can match more than one place, and the walk
/// follows all of them.
pub fn find_items_by_path(conn: &Connection, path: &str) -> Result<Vec<Item>> {
    let mut parts = path.split('/').map(str::trim).filter(|s| !s.is_empty());

    let Some(first) = parts.next() else {
        return Ok(Vec::new());
    };
    let mut matches = find_items_named_in(conn, first, None)?;

    for part in parts {
        let mut next = Vec::new();
        for place in &matches {
            next.extend(find_items_named_in(conn, part, Some(place.id))?);
        }
        matches = next;
    }

    Ok(matches)
}

/// Get the path to an item as a vector of names (from root to item).
pub fn get_item_path(conn: &Connection, item_id: i64) -> Result<Vec<String>> {
    let mut path = Vec::new();
    let mut current_id = Some(item_id);

    while let Some(id) = current_id {
        if let Some(item) = get_item_by_id(conn, id)? {
            path.push(item.name);
            current_id = item.place_id;
        } else {
            break;
        }
    }

    path.reverse();
    Ok(path)
}

/// List items at root level (no place).
pub fn list_root_items(conn: &Connection) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, place_id, kind, created_at, updated_at
         FROM items WHERE place_id IS NULL",
    )?;

    let items = stmt
        .query_map([], item_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// List items in a specific place.
pub fn list_items_in_place(conn: &Connection, place_id: i64) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, place_id, kind, created_at, updated_at
         FROM items WHERE place_id = ?1",
    )?;

    let items = stmt
        .query_map(params![place_id], item_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// List all items recursively.
pub fn list_all_items(conn: &Connection) -> Result<Vec<Item>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, place_id, kind, created_at, updated_at FROM items",
    )?;

    let items = stmt
        .query_map([], item_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

/// Count children of an item.
pub fn count_children(conn: &Connection, item_id: i64) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM items WHERE place_id = ?1",
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

/// Update an item's kind.
pub fn update_item_kind(conn: &Connection, item_id: i64, kind: Kind) -> Result<()> {
    conn.execute(
        "UPDATE items SET kind = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![kind, item_id],
    )?;
    Ok(())
}

/// Move an item to a new place.
pub fn move_item(conn: &Connection, item_id: i64, new_place_id: Option<i64>) -> Result<()> {
    conn.execute(
        "UPDATE items SET place_id = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![new_place_id, item_id],
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
            current_id = item.place_id;
        } else {
            break;
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Write a database in the version 1 schema, before place_id existed.
    fn v1_database(path: &Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                container_id INTEGER REFERENCES items(id) ON DELETE SET NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX idx_items_name ON items(name);
            CREATE INDEX idx_items_container ON items(container_id);
            CREATE UNIQUE INDEX idx_items_name_container
                ON items(name, COALESCE(container_id, 0));

            INSERT INTO items (id, name, description, container_id)
                VALUES (1, 'garage', NULL, NULL),
                       (2, 'toolbox', NULL, 1),
                       (3, 'hammer', '16oz claw', 2);
            "#,
        )
        .unwrap();
        conn
    }

    fn index_names(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%' ORDER BY name")
            .unwrap();
        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        names
    }

    fn user_version(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn a_version_1_database_gains_place_id_and_keeps_its_rows() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("old.db");
        drop(v1_database(&path));

        let conn = open(Some(&path)).unwrap();

        assert!(has_column(&conn, "items", "place_id").unwrap());
        assert!(!has_column(&conn, "items", "container_id").unwrap());
        assert_eq!(user_version(&conn), SCHEMA_VERSION);

        let hammer = get_item_by_id(&conn, 3).unwrap().unwrap();
        assert_eq!(hammer.name, "hammer");
        assert_eq!(hammer.description.as_deref(), Some("16oz claw"));
        assert_eq!(hammer.place_id, Some(2));
        assert_eq!(
            get_item_path(&conn, 3).unwrap(),
            ["garage", "toolbox", "hammer"]
        );
    }

    #[test]
    fn migrating_replaces_the_old_index_names() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("old.db");
        drop(v1_database(&path));

        let conn = open(Some(&path)).unwrap();

        assert_eq!(
            index_names(&conn),
            ["idx_items_kind", "idx_items_name", "idx_items_place"]
        );
    }

    #[test]
    fn migrating_twice_changes_nothing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("old.db");
        drop(v1_database(&path));

        drop(open(Some(&path)).unwrap());
        let conn = open(Some(&path)).unwrap();

        assert_eq!(user_version(&conn), SCHEMA_VERSION);
        assert_eq!(list_all_items(&conn).unwrap().len(), 3);
    }

    /// Write a database in the version 2 schema, before kind existed.
    fn v2_database(path: &Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                place_id INTEGER REFERENCES items(id) ON DELETE SET NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX idx_items_name ON items(name);
            CREATE INDEX idx_items_place ON items(place_id);
            CREATE UNIQUE INDEX idx_items_name_place
                ON items(name, COALESCE(place_id, 0));

            INSERT INTO items (id, name, place_id) VALUES (1, 'garage', NULL);
            PRAGMA user_version = 2;
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn a_version_2_database_gains_kind_with_every_row_a_thing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("v2.db");
        drop(v2_database(&path));

        let conn = open(Some(&path)).unwrap();

        assert_eq!(user_version(&conn), SCHEMA_VERSION);
        assert_eq!(get_item_by_id(&conn, 1).unwrap().unwrap().kind, Kind::Thing);
    }

    /// Write a database in the version 3 schema, with names unique per place.
    fn v3_database(path: &Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                place_id INTEGER REFERENCES items(id) ON DELETE SET NULL,
                kind TEXT NOT NULL DEFAULT 'thing',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX idx_items_name ON items(name);
            CREATE INDEX idx_items_kind ON items(kind);
            CREATE INDEX idx_items_place ON items(place_id);
            CREATE UNIQUE INDEX idx_items_name_place
                ON items(name, COALESCE(place_id, 0));

            INSERT INTO items (id, name, place_id) VALUES (1, 'toolbox', NULL),
                                                          (2, 'hammer', 1);
            PRAGMA user_version = 3;
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn a_version_3_database_allows_the_same_name_twice_in_a_place() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("v3.db");
        drop(v3_database(&path));

        let conn = open(Some(&path)).unwrap();

        assert_eq!(user_version(&conn), SCHEMA_VERSION);
        assert!(insert_item(&conn, "hammer", None, Some(1), Kind::Thing).is_ok());
        assert_eq!(
            index_names(&conn),
            ["idx_items_kind", "idx_items_name", "idx_items_place"]
        );
    }

    #[test]
    fn an_unknown_kind_in_the_database_reads_as_a_thing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("odd.db");
        let conn = open(Some(&path)).unwrap();
        insert_item(&conn, "garage", None, None, Kind::Room).unwrap();

        conn.execute(
            "UPDATE items SET kind = 'wardrobe' WHERE name = 'garage'",
            [],
        )
        .unwrap();

        assert_eq!(get_item_by_id(&conn, 1).unwrap().unwrap().kind, Kind::Thing);
    }

    #[test]
    fn a_new_database_starts_at_the_current_version() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("new.db");

        let conn = open(Some(&path)).unwrap();

        assert_eq!(user_version(&conn), SCHEMA_VERSION);
        assert!(has_column(&conn, "items", "place_id").unwrap());
        assert_eq!(
            index_names(&conn),
            ["idx_items_kind", "idx_items_name", "idx_items_place"]
        );
    }
}
