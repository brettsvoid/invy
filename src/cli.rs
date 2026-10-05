//! CLI argument definitions for invy.
//!
//! See SPEC.md for full behavioral specification.

use crate::model::{GlyphSet, Kind};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// A CLI tool for tracking home inventory with hierarchical places.
///
/// See SPEC.md for full documentation.
#[derive(Parser, Debug)]
#[command(name = "invy")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// What to do. With none, invy opens the TUI
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output as JSON
    #[arg(short, long, global = true)]
    pub json: bool,

    /// Output as CSV
    #[arg(long, global = true)]
    pub csv: bool,

    /// Use custom database file
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    /// Characters to draw the tree with
    ///
    /// "nerd" needs a patched font. "unicode" works anywhere, "ascii" is for pipes.
    #[arg(long, global = true, value_enum, env = "INVY_GLYPHS", default_value_t = GlyphSet::Nerd)]
    pub glyphs: GlyphSet,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new item to the inventory
    ///
    /// See SPEC.md#invy-add-name
    Add {
        /// Name of the item
        name: String,

        /// Item description
        #[arg(short, long)]
        desc: Option<String>,

        /// Place to put the item in (auto-creates if needed)
        #[arg(short = 'i', long = "in")]
        place: Option<String>,

        /// What sort of thing this is
        #[arg(short, long, value_enum, default_value_t = Kind::Thing)]
        kind: Kind,

        /// How many to add. Each one is its own item
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
        count: u32,
    },

    /// Search for items by name or description
    ///
    /// See SPEC.md#invy-find-query
    Find {
        /// Search in fzf's syntax: words match fuzzily in any order. 'word is
        /// exact, ^word a prefix, word$ a suffix, !word excludes
        ///
        /// Optional when --kind is given.
        query: Option<String>,

        /// Only show items of this kind
        #[arg(short, long, value_enum)]
        kind: Option<Kind>,
    },

    /// List items, optionally within a specific place
    ///
    /// See SPEC.md#invy-list-place
    List {
        /// Place to list (default: root)
        place: Option<String>,

        /// List all descendants recursively
        #[arg(short, long)]
        recursive: bool,
    },

    /// Show detailed information about a specific item
    ///
    /// See SPEC.md#invy-show-item
    Show {
        /// Item name or path
        item: String,
    },

    /// Move an item to a different place
    ///
    /// See SPEC.md#invy-mv-item-destination
    Mv {
        /// Item to move
        item: String,

        /// Target place (use "/" for root)
        destination: String,

        /// Move every duplicate the name matches, not just one
        #[arg(long)]
        all: bool,
    },

    /// Remove an item from the inventory
    ///
    /// See SPEC.md#invy-rm-item
    Rm {
        /// Item to remove
        item: String,

        /// Remove every duplicate the name matches, not just one
        #[arg(long)]
        all: bool,
    },

    /// Browse and edit the inventory in an interactive terminal UI
    #[command(alias = "ui")]
    Tui,

    /// Edit an existing item's name, description or kind
    ///
    /// See SPEC.md#invy-edit-item
    Edit {
        /// Item to edit
        item: String,

        /// New name
        #[arg(short, long)]
        name: Option<String>,

        /// New description (use "" to clear)
        #[arg(short, long)]
        desc: Option<String>,

        /// New kind
        #[arg(short, long, value_enum)]
        kind: Option<Kind>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_subcommand_is_allowed() {
        assert!(Cli::try_parse_from(["invy"]).unwrap().command.is_none());
        assert!(Cli::try_parse_from(["invy", "--db", "x.db"])
            .unwrap()
            .command
            .is_none());
    }
}
