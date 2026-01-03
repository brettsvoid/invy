//! Data models for invy.

use serde::{Deserialize, Serialize};

/// An item in the inventory.
///
/// Items can be standalone or nested inside containers.
/// A container is just an item that has other items inside it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<i64>,
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
}

impl Item {
    /// Convert into ListItem with child count.
    pub fn into_list_item(self, child_count: i64) -> ListItem {
        ListItem {
            id: self.id,
            name: self.name,
            description: self.description,
            child_count,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreeItem>,
}
