//! invy - A CLI tool for tracking home inventory with hierarchical containers.
//!
//! See SPEC.md for full behavioral specification.

mod cli;
mod commands;
mod db;
mod model;
mod output;

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
            container,
        } => commands::add::run(
            &name,
            desc.as_deref(),
            container.as_deref(),
            cli.json,
            cli.csv,
            db_path,
        ),

        Commands::Find { query } => commands::find::run(&query, cli.json, cli.csv, db_path),

        Commands::List {
            container,
            recursive,
        } => commands::list::run(container.as_deref(), recursive, cli.json, cli.csv, db_path),

        Commands::Show { item } => commands::show::run(&item, cli.json, cli.csv, db_path),

        Commands::Mv { item, destination } => {
            commands::mv::run(&item, &destination, cli.json, cli.csv, db_path)
        }

        Commands::Rm { item } => commands::rm::run(&item, cli.json, cli.csv, db_path),

        Commands::Edit { item, name, desc } => commands::edit::run(
            &item,
            name.as_deref(),
            desc.as_deref(),
            cli.json,
            cli.csv,
            db_path,
        ),
    }
}
