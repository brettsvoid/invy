//! invy - A CLI tool for tracking home inventory with hierarchical places.
//!
//! See SPEC.md for full behavioral specification.

mod cli;
mod commands;
mod config;
mod db;
mod inventory;
mod model;
mod output;
mod search;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    model::set_glyph_set(cli.glyphs);

    let config = config::load()?;
    let db = cli.db.or(config.db);
    let db_path = db.as_deref();

    // With no subcommand, open the TUI.
    let Some(command) = cli.command else {
        return tui::run(db_path);
    };

    match command {
        Commands::Add {
            name,
            desc,
            place,
            kind,
            count,
        } => commands::add::run(
            &name,
            desc.as_deref(),
            place.as_deref(),
            kind,
            count,
            cli.json,
            cli.csv,
            db_path,
        ),

        Commands::Find { query, kind } => {
            commands::find::run(query.as_deref(), kind, cli.json, cli.csv, db_path)
        }

        Commands::List { place, recursive } => {
            commands::list::run(place.as_deref(), recursive, cli.json, cli.csv, db_path)
        }

        Commands::Show { item } => commands::show::run(&item, cli.json, cli.csv, db_path),

        Commands::Mv {
            item,
            destination,
            all,
        } => commands::mv::run(&item, &destination, all, cli.json, cli.csv, db_path),

        Commands::Rm { item, all } => commands::rm::run(&item, all, cli.json, cli.csv, db_path),

        Commands::Tui => tui::run(db_path),

        Commands::Edit {
            item,
            name,
            desc,
            kind,
        } => commands::edit::run(
            &item,
            name.as_deref(),
            desc.as_deref(),
            kind,
            cli.json,
            cli.csv,
            db_path,
        ),
    }
}
