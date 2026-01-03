//! CLI argument definitions for invy.
//!
//! See SPEC.md for full behavioral specification.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// A CLI tool for tracking home inventory with hierarchical containers.
///
/// See SPEC.md for full documentation.
#[derive(Parser, Debug)]
#[command(name = "invy")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output as JSON
    #[arg(short, long, global = true)]
    pub json: bool,

    /// Output as CSV
    #[arg(long, global = true)]
    pub csv: bool,

    /// Use custom database file
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,
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

        /// Container to place item in (auto-creates if needed)
        #[arg(short = 'i', long = "in")]
        container: Option<String>,
    },

    /// Search for items by name or description
    ///
    /// See SPEC.md#invy-find-query
    Find {
        /// Search term (substring match, case-insensitive)
        query: String,
    },

    /// List items, optionally within a specific container
    ///
    /// See SPEC.md#invy-list-container
    List {
        /// Container to list (default: root)
        container: Option<String>,

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

    /// Move an item to a different container
    ///
    /// See SPEC.md#invy-mv-item-destination
    Mv {
        /// Item to move
        item: String,

        /// Target container (use "/" for root)
        destination: String,
    },

    /// Remove an item from the inventory
    ///
    /// See SPEC.md#invy-rm-item
    Rm {
        /// Item to remove
        item: String,
    },

    /// Edit an existing item's name or description
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
    },
}
