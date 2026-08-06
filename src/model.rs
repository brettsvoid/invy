//! Data models for invy.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// What sort of thing an item is.
///
/// The first three describe a place. `Thing` is the default, and covers
/// everything you put in one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A room, a loft, a shed, a garden
    Room,
    /// A cupboard, a dresser, a shelf, a workbench
    Furniture,
    /// A box, a bag, a case, a toolbox
    Box,
    /// Anything you put in a place
    #[default]
    Thing,
}

impl Kind {
    /// Every kind, in the order the TUI cycles through them.
    pub const ALL: [Kind; 4] = [Kind::Room, Kind::Furniture, Kind::Box, Kind::Thing];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Room => "room",
            Kind::Furniture => "furniture",
            Kind::Box => "box",
            Kind::Thing => "thing",
        }
    }

    /// Read a kind written by the database.
    ///
    /// An unrecognised value falls back to `Thing`. The kind is descriptive, so
    /// a hand-edited row should not stop the app from opening.
    pub fn from_db(value: &str) -> Self {
        match value {
            "room" => Kind::Room,
            "furniture" => Kind::Furniture,
            "box" => Kind::Box,
            _ => Kind::Thing,
        }
    }

    /// A marker for the TUI tree. `Thing` has none, so ordinary items stay plain.
    pub fn glyph(self) -> Option<&'static str> {
        match self {
            Kind::Room => Some("⌂"),
            Kind::Furniture => Some("▤"),
            Kind::Box => Some("▣"),
            Kind::Thing => None,
        }
    }

    /// The next kind, wrapping round.
    pub fn next(self) -> Self {
        let index = Kind::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Kind::ALL[(index + 1) % Kind::ALL.len()]
    }

    /// The previous kind, wrapping round.
    pub fn previous(self) -> Self {
        let index = Kind::ALL.iter().position(|k| *k == self).unwrap_or(0);
        Kind::ALL[(index + Kind::ALL.len() - 1) % Kind::ALL.len()]
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl rusqlite::ToSql for Kind {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.as_str().into())
    }
}

/// An item in the inventory.
///
/// Items can be standalone or nested inside places.
/// A place is just an item that has other items inside it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place_id: Option<i64>,
    pub kind: Kind,
    pub created_at: String,
    pub updated_at: String,
}

/// An item with its full path and child count for display purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemWithPath {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub path: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_count: Option<i64>,
    pub kind: Kind,
    pub created_at: String,
    pub updated_at: String,
}

impl Item {
    /// Convert to ItemWithPath with the given path and child count.
    pub fn with_path(self, path: Vec<String>, child_count: Option<i64>) -> ItemWithPath {
        ItemWithPath {
            id: self.id,
            name: self.name,
            description: self.description,
            path,
            child_count,
            kind: self.kind,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Item for list display (with child count).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub child_count: i64,
    pub kind: Kind,
}

impl Item {
    /// Convert into ListItem with child count.
    pub fn into_list_item(self, child_count: i64) -> ListItem {
        ListItem {
            id: self.id,
            name: self.name,
            description: self.description,
            child_count,
            kind: self.kind,
        }
    }
}

/// Item with nested children for tree display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeItem {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub child_count: i64,
    pub kind: Kind,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreeItem>,
}
