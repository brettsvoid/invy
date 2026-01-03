//! Output formatting for invy.
//!
//! Supports human-readable, JSON, and CSV output formats.

use anyhow::Result;
use serde::Serialize;
use std::io;

use crate::model::{ItemWithPath, ListItem, TreeItem};

/// Output format selection.
#[derive(Debug, Clone, Copy)]
pub enum Format {
    Human,
    Json,
    Csv,
}

impl Format {
    /// Create format from CLI flags.
    pub fn from_flags(json: bool, csv: bool) -> Self {
        if json {
            Format::Json
        } else if csv {
            Format::Csv
        } else {
            Format::Human
        }
    }
}

/// Output a single item (for add, show commands).
pub fn print_item(item: &ItemWithPath, format: Format) -> Result<()> {
    match format {
        Format::Human => print_item_human(item),
        Format::Json => print_json(item),
        Format::Csv => print_item_csv(item),
    }
}

/// Output a list of items (for find, list commands).
pub fn print_items(items: &[ItemWithPath], format: Format) -> Result<()> {
    match format {
        Format::Human => print_items_human(items),
        Format::Json => print_json(items),
        Format::Csv => print_items_csv(items),
    }
}

/// Output list items with child counts (for list command).
pub fn print_list_items(items: &[ListItem], format: Format) -> Result<()> {
    match format {
        Format::Human => print_list_items_human(items),
        Format::Json => print_json(items),
        Format::Csv => print_list_items_csv(items),
    }
}

/// Print added item message.
pub fn print_added(item: &ItemWithPath, format: Format) -> Result<()> {
    match format {
        Format::Human => {
            println!("Added: {}", item.name);
            if item.path.len() > 1 {
                let container_path = &item.path[..item.path.len() - 1];
                println!("  -> {}", container_path.join(" -> "));
            }
            Ok(())
        }
        Format::Json => print_json(item),
        Format::Csv => {
            println!("id,name,description,container");
            println!(
                "{},{},{},{}",
                item.id,
                item.name,
                item.description.as_deref().unwrap_or(""),
                if item.path.len() > 1 {
                    item.path[item.path.len() - 2].clone()
                } else {
                    String::new()
                }
            );
            Ok(())
        }
    }
}

/// Print moved item message.
pub fn print_moved(item: &ItemWithPath, old_path: &[String], format: Format) -> Result<()> {
    match format {
        Format::Human => {
            println!("Moved: {}", item.name);
            if old_path.len() > 1 {
                println!(
                    "  {} -> {}",
                    old_path[..old_path.len() - 1].join(" -> "),
                    if item.path.len() > 1 {
                        item.path[..item.path.len() - 1].join(" -> ")
                    } else {
                        "(root)".to_string()
                    }
                );
            } else {
                println!(
                    "  (root) -> {}",
                    if item.path.len() > 1 {
                        item.path[..item.path.len() - 1].join(" -> ")
                    } else {
                        "(root)".to_string()
                    }
                );
            }
            Ok(())
        }
        Format::Json => print_json(item),
        Format::Csv => print_item_csv(item),
    }
}

/// Print removed item message.
pub fn print_removed(name: &str, orphaned: &[String], format: Format) -> Result<()> {
    match format {
        Format::Human => {
            println!("Removed: {}", name);
            if !orphaned.is_empty() {
                println!("Orphaned {} items to root:", orphaned.len());
                for item_name in orphaned {
                    println!("  - {}", item_name);
                }
            }
            Ok(())
        }
        Format::Json => {
            #[derive(Serialize)]
            struct RemovedOutput {
                removed: String,
                orphaned: Vec<String>,
            }
            print_json(&RemovedOutput {
                removed: name.to_string(),
                orphaned: orphaned.to_vec(),
            })
        }
        Format::Csv => {
            println!("removed,orphaned");
            println!("{},{}", name, orphaned.join(";"));
            Ok(())
        }
    }
}

/// Print updated item message.
pub fn print_updated(
    item: &ItemWithPath,
    old_name: Option<&str>,
    old_desc: Option<Option<&str>>,
    format: Format,
) -> Result<()> {
    match format {
        Format::Human => {
            print!("Updated: {}", item.name);
            if let Some(old) = old_name {
                if old != item.name {
                    print!(" (was: {})", old);
                }
            }
            println!();

            if let Some(old_d) = old_desc {
                let new_d = item.description.as_deref();
                if old_d != new_d {
                    println!(
                        "  description: {:?} -> {:?}",
                        old_d.unwrap_or("(none)"),
                        new_d.unwrap_or("(none)")
                    );
                }
            }
            Ok(())
        }
        Format::Json => print_json(item),
        Format::Csv => print_item_csv(item),
    }
}

// Human-readable formatters

fn print_item_human(item: &ItemWithPath) -> Result<()> {
    println!("Name:        {}", item.name);
    println!(
        "Description: {}",
        item.description.as_deref().unwrap_or("-")
    );

    if item.path.len() > 1 {
        let location = &item.path[..item.path.len() - 1];
        let mut reversed = location.to_vec();
        reversed.reverse();
        println!("Location:    {}", reversed.join(" -> "));
    } else {
        println!("Location:    (root)");
    }

    if let Some(count) = item.child_count {
        if count > 0 {
            println!("Contains:    {} items", count);
        }
    }

    println!("Created:     {}", item.created_at);
    println!("Updated:     {}", item.updated_at);

    Ok(())
}

fn print_items_human(items: &[ItemWithPath]) -> Result<()> {
    for item in items {
        print!("{}", item.name);
        if let Some(ref desc) = item.description {
            print!(" ({})", desc);
        }
        println!();

        if item.path.len() > 1 {
            let location = &item.path[..item.path.len() - 1];
            let mut reversed = location.to_vec();
            reversed.reverse();
            println!("  -> {}", reversed.join(" -> "));
        }
        println!();
    }
    Ok(())
}

fn print_list_items_human(items: &[ListItem]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    // Calculate column widths
    let max_name = items.iter().map(|i| i.name.len()).max().unwrap_or(4).max(4);
    let max_desc = items
        .iter()
        .map(|i| i.description.as_ref().map(|d| d.len()).unwrap_or(1))
        .max()
        .unwrap_or(11)
        .max(11);

    // Header
    println!(
        "{:<width_name$} {:<width_desc$} ITEMS",
        "NAME",
        "DESCRIPTION",
        width_name = max_name,
        width_desc = max_desc
    );

    // Rows
    for item in items {
        let desc = item.description.as_deref().unwrap_or("-");
        let items_str = if item.child_count > 0 {
            item.child_count.to_string()
        } else {
            "-".to_string()
        };
        println!(
            "{:<width_name$} {:<width_desc$} {}",
            item.name,
            desc,
            items_str,
            width_name = max_name,
            width_desc = max_desc
        );
    }

    Ok(())
}

// JSON formatter

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    let json = serde_json::to_string(value)?;
    println!("{}", json);
    Ok(())
}

// CSV formatters

fn print_item_csv(item: &ItemWithPath) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(["id", "name", "description", "path"])?;
    wtr.write_record([
        &item.id.to_string(),
        &item.name,
        item.description.as_deref().unwrap_or(""),
        &item.path.join("/"),
    ])?;
    wtr.flush()?;
    Ok(())
}

fn print_items_csv(items: &[ItemWithPath]) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(["id", "name", "description", "path"])?;
    for item in items {
        wtr.write_record([
            &item.id.to_string(),
            &item.name,
            item.description.as_deref().unwrap_or(""),
            &item.path.join("/"),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

fn print_list_items_csv(items: &[ListItem]) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(["id", "name", "description", "child_count"])?;
    for item in items {
        wtr.write_record([
            &item.id.to_string(),
            &item.name,
            item.description.as_deref().unwrap_or(""),
            &item.child_count.to_string(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

// Tree output (for recursive list)

/// Output tree items with hierarchy (for recursive list command).
pub fn print_tree_items(items: &[TreeItem], format: Format) -> Result<()> {
    match format {
        Format::Human => print_tree_items_human(items),
        Format::Json => print_json(items),
        Format::Csv => print_tree_items_csv(items),
    }
}

/// Tree rendering characters
const TREE_BRANCH: &str = "├── ";
const TREE_LAST: &str = "└── ";
const TREE_VERTICAL: &str = "│   ";
const TREE_SPACE: &str = "    ";

fn print_tree_items_human(items: &[TreeItem]) -> Result<()> {
    fn print_item_line(item: &TreeItem) {
        print!("{}", item.name);
        if let Some(ref desc) = item.description {
            print!(" ({})", desc);
        }
        if item.child_count > 0 {
            print!(" [{}]", item.child_count);
        }
        println!();
    }

    fn print_subtree(item: &TreeItem, prefix: &str, is_last: bool) {
        let connector = if is_last { TREE_LAST } else { TREE_BRANCH };

        print!("{}{}", prefix, connector);
        print_item_line(item);

        let child_prefix = format!(
            "{}{}",
            prefix,
            if is_last { TREE_SPACE } else { TREE_VERTICAL }
        );

        let child_count = item.children.len();
        for (i, child) in item.children.iter().enumerate() {
            print_subtree(child, &child_prefix, i == child_count - 1);
        }
    }

    for item in items {
        // Root items: print without prefix
        print_item_line(item);

        // Print children with tree structure
        let child_count = item.children.len();
        for (i, child) in item.children.iter().enumerate() {
            print_subtree(child, "", i == child_count - 1);
        }
    }

    Ok(())
}

fn print_tree_items_csv(items: &[TreeItem]) -> Result<()> {
    // Flatten tree for CSV output
    fn collect_flat(items: &[TreeItem], result: &mut Vec<ListItem>) {
        for item in items {
            result.push(ListItem {
                id: item.id,
                name: item.name.clone(),
                description: item.description.clone(),
                child_count: item.child_count,
            });
            collect_flat(&item.children, result);
        }
    }

    let mut flat_items = Vec::new();
    collect_flat(items, &mut flat_items);
    print_list_items_csv(&flat_items)
}
