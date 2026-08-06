//! Data models for invy.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Which icons to draw beside a place.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum GlyphSet {
    /// Nerd Font icons. Needs a patched font, or they show as blank boxes.
    #[default]
    Nerd,
    /// Geometric shapes from the base Unicode planes. Works in any font.
    Unicode,
    /// No icons at all. Use this when piping to something that cannot draw them.
    Ascii,
}

static GLYPH_SET: OnceLock<GlyphSet> = OnceLock::new();

/// Fix the glyph set for this run. Only the first call takes effect.
pub fn set_glyph_set(set: GlyphSet) {
    let _ = GLYPH_SET.set(set);
}

/// The glyph set chosen for this run.
pub fn glyph_set() -> GlyphSet {
    GLYPH_SET.get().copied().unwrap_or_default()
}

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

    /// A marker for the tree, in the glyph set chosen for this run.
    ///
    /// `Thing` never has one, so ordinary items stay plain.
    pub fn glyph(self) -> Option<&'static str> {
        self.glyph_in(glyph_set())
    }

    /// A marker for the tree, in a named glyph set.
    ///
    /// The Nerd Font codepoints are `md-home_variant`, `md-dresser` and
    /// `md-package_variant_closed`. They live in the Private Use Area, so a
    /// terminal without a patched font draws blank boxes instead.
    pub fn glyph_in(self, set: GlyphSet) -> Option<&'static str> {
        match (set, self) {
            (_, Kind::Thing) | (GlyphSet::Ascii, _) => None,

            (GlyphSet::Nerd, Kind::Room) => Some("\u{f02de}"),
            (GlyphSet::Nerd, Kind::Furniture) => Some("\u{f0f4a}"),
            (GlyphSet::Nerd, Kind::Box) => Some("\u{f03d7}"),

            (GlyphSet::Unicode, Kind::Room) => Some("⌂"),
            (GlyphSet::Unicode, Kind::Furniture) => Some("▤"),
            (GlyphSet::Unicode, Kind::Box) => Some("▣"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_set_leaves_a_thing_unmarked() {
        for set in [GlyphSet::Nerd, GlyphSet::Unicode, GlyphSet::Ascii] {
            assert_eq!(Kind::Thing.glyph_in(set), None);
        }
    }

    #[test]
    fn the_ascii_set_marks_nothing_at_all() {
        for kind in Kind::ALL {
            assert_eq!(kind.glyph_in(GlyphSet::Ascii), None);
        }
    }

    #[test]
    fn each_place_kind_has_its_own_glyph_in_the_drawing_sets() {
        for set in [GlyphSet::Nerd, GlyphSet::Unicode] {
            let marks: Vec<&str> = [Kind::Room, Kind::Furniture, Kind::Box]
                .iter()
                .map(|k| k.glyph_in(set).expect("a place is marked"))
                .collect();

            let mut unique = marks.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), marks.len(), "{set:?} reuses a glyph");
        }
    }

    #[test]
    fn the_nerd_glyphs_sit_in_the_private_use_area() {
        // Anything outside it would not be a Nerd Font icon.
        for kind in [Kind::Room, Kind::Furniture, Kind::Box] {
            let glyph = kind.glyph_in(GlyphSet::Nerd).unwrap();
            let cp = glyph.chars().next().unwrap() as u32;
            assert!(
                (0xE000..=0xF8FF).contains(&cp) || (0xF0000..=0xFFFFD).contains(&cp),
                "{kind} maps to U+{cp:04X}, outside the Private Use Area"
            );
            assert_eq!(glyph.chars().count(), 1, "{kind} is not a single glyph");
        }
    }

    #[test]
    fn kinds_cycle_in_both_directions_and_wrap() {
        assert_eq!(Kind::Room.next(), Kind::Furniture);
        assert_eq!(Kind::Thing.next(), Kind::Room);
        assert_eq!(Kind::Room.previous(), Kind::Thing);
        assert_eq!(Kind::Furniture.previous(), Kind::Room);
    }
}
