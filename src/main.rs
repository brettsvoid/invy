//! invy - A CLI tool for tracking home inventory with hierarchical places.
//!
//! See SPEC.md for full behavioral specification.

mod cli;
mod commands;
mod db;
mod model;
mod output;
mod tui;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let db_path = cli.db.as_deref();

    match cli.command {
        Commands::Add {
            name,
            desc,
            place,
            kind,
        } => commands::add::run(
            &name,
            desc.as_deref(),
            place.as_deref(),
            kind,
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

        Commands::Mv { item, destination } => {
            commands::mv::run(&item, &destination, cli.json, cli.csv, db_path)
        }

        Commands::Rm { item } => commands::rm::run(&item, cli.json, cli.csv, db_path),

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
