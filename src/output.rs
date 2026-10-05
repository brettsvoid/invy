//! Output formatting for invy.
//!
//! Supports human-readable, JSON, and CSV output formats.

use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::hash::Hash;
use std::io;

use crate::model::{glyph_set, ItemWithPath, Kind, ListItem, TreeItem};

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

/// Output a single item (for the show command).
///
/// `duplicates` counts the item and its duplicates. Only human output shows it.
pub fn print_item(item: &ItemWithPath, duplicates: usize, format: Format) -> Result<()> {
    match format {
        Format::Human => print_item_human(item, duplicates),
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

/// Say how many items a command acted on, after its verb: " 3", or " 1 of 3"
/// when it took one of several duplicates. Nothing when only one matched.
fn counted(done: usize, matched: usize) -> String {
    if matched <= 1 {
        String::new()
    } else if done == matched {
        format!(" {done}")
    } else {
        format!(" {done} of {matched}")
    }
}

/// Print one JSON object for a single item, or an array for several.
fn print_json_one_or_many(items: &[ItemWithPath]) -> Result<()> {
    match items {
        [item] => print_json(item),
        _ => print_json(items),
    }
}

/// Print added items message. Several items are duplicates of one another.
pub fn print_added(items: &[ItemWithPath], format: Format) -> Result<()> {
    let item = &items[0];
    match format {
        Format::Human => {
            println!("Added{}: {}", counted(items.len(), items.len()), item.name);
            if item.path.len() > 1 {
                let place_path = &item.path[..item.path.len() - 1];
                println!("  -> {}", place_path.join(" -> "));
            }
            Ok(())
        }
        Format::Json => print_json_one_or_many(items),
        Format::Csv => {
            println!("id,name,description,kind,place");
            for item in items {
                println!(
                    "{},{},{},{},{}",
                    item.id,
                    item.name,
                    item.description.as_deref().unwrap_or(""),
                    item.kind,
                    if item.path.len() > 1 {
                        item.path[item.path.len() - 2].clone()
                    } else {
                        String::new()
                    }
                );
            }
            Ok(())
        }
    }
}

/// Print moved items message. Several items are duplicates of one another.
pub fn print_moved(
    items: &[ItemWithPath],
    matched: usize,
    old_path: &[String],
    format: Format,
) -> Result<()> {
    let item = &items[0];
    match format {
        Format::Human => {
            println!("Moved{}: {}", counted(items.len(), matched), item.name);
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
        Format::Json => print_json_one_or_many(items),
        Format::Csv => print_items_csv(items),
    }
}

/// Print removed items message.
pub fn print_removed(
    name: &str,
    removed: usize,
    matched: usize,
    orphaned: &[String],
    format: Format,
) -> Result<()> {
    match format {
        Format::Human => {
            println!("Removed{}: {}", counted(removed, matched), name);
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
                count: usize,
                orphaned: Vec<String>,
            }
            print_json(&RemovedOutput {
                removed: name.to_string(),
                count: removed,
                orphaned: orphaned.to_vec(),
            })
        }
        Format::Csv => {
            println!("removed,orphaned,count");
            println!("{},{},{}", name, orphaned.join(";"), removed);
            Ok(())
        }
    }
}

/// Print updated item message.
pub fn print_updated(
    item: &ItemWithPath,
    old_name: Option<&str>,
    old_desc: Option<Option<&str>>,
    old_kind: Option<Kind>,
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

            if let Some(old_k) = old_kind {
                if old_k != item.kind {
                    println!("  kind: {} -> {}", old_k, item.kind);
                }
            }
            Ok(())
        }
        Format::Json => print_json(item),
        Format::Csv => print_item_csv(item),
    }
}

// Human-readable formatters

/// Fold duplicates together for human output, keeping first-seen order.
///
/// `key` gives what two duplicates share, or `None` for an item that holds
/// something and so is never a duplicate. Each entry comes back with how many
/// items it stands for.
fn group_duplicates<T, K: Eq + Hash>(
    items: &[T],
    key: impl Fn(&T) -> Option<K>,
) -> Vec<(&T, usize)> {
    let mut groups: Vec<(&T, usize)> = Vec::new();
    let mut index: HashMap<K, usize> = HashMap::new();
    for item in items {
        match key(item) {
            Some(k) => match index.get(&k) {
                Some(&i) => groups[i].1 += 1,
                None => {
                    index.insert(k, groups.len());
                    groups.push((item, 1));
                }
            },
            None => groups.push((item, 1)),
        }
    }
    groups
}

/// A name with its duplicate count, as in `hdmi cable ×3`.
fn counted_name(name: &str, count: usize) -> String {
    if count > 1 {
        format!("{name} {}{count}", glyph_set().times())
    } else {
        name.to_string()
    }
}

/// What two duplicates in one place share. Names compare as SQLite's NOCASE.
fn sibling_key(
    name: &str,
    description: &Option<String>,
    kind: Kind,
) -> (String, Option<String>, Kind) {
    (name.to_ascii_lowercase(), description.clone(), kind)
}

fn print_item_human(item: &ItemWithPath, duplicates: usize) -> Result<()> {
    println!("Name:        {}", item.name);
    println!(
        "Description: {}",
        item.description.as_deref().unwrap_or("-")
    );

    if item.path.len() > 1 {
        let location = &item.path[..item.path.len() - 1];
        let mut reversed = location.to_vec();
        reversed.reverse();
        println!("Place:       {}", reversed.join(" -> "));
    } else {
        println!("Place:       (root)");
    }

    if let Some(count) = item.child_count {
        if count > 0 {
            println!("Contains:    {} items", count);
        }
    }

    if duplicates > 1 {
        println!("Duplicates:  {} here", duplicates);
    }

    println!("Kind:        {}", item.kind);
    println!("Created:     {}", item.created_at);
    println!("Updated:     {}", item.updated_at);

    Ok(())
}

fn print_items_human(items: &[ItemWithPath]) -> Result<()> {
    let groups = group_duplicates(items, |item| {
        (item.child_count == Some(0)).then(|| {
            (
                item.place_id,
                sibling_key(&item.name, &item.description, item.kind),
            )
        })
    });
    for (item, count) in groups {
        println!("{}", counted_name(&item.path.join("/"), count));
        if let Some(ref desc) = item.description {
            println!("  {}", desc);
        }
        println!();
    }
    Ok(())
}

fn print_list_items_human(items: &[ListItem]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    let rows: Vec<(String, &ListItem)> = group_duplicates(items, |item| {
        (item.child_count == 0).then(|| sibling_key(&item.name, &item.description, item.kind))
    })
    .into_iter()
    .map(|(item, count)| (counted_name(&item.name, count), item))
    .collect();

    // Calculate column widths. Rust pads by characters, so count characters.
    let max_name = rows
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let max_desc = rows
        .iter()
        .map(|(_, i)| i.description.as_ref().map_or(1, |d| d.chars().count()))
        .max()
        .unwrap_or(11)
        .max(11);

    // Header
    let max_kind = rows
        .iter()
        .map(|(_, i)| i.kind.as_str().len())
        .max()
        .unwrap_or(4)
        .max(4);

    println!(
        "{:<width_name$} {:<width_kind$} {:<width_desc$} ITEMS",
        "NAME",
        "KIND",
        "DESCRIPTION",
        width_name = max_name,
        width_kind = max_kind,
        width_desc = max_desc
    );

    // Rows
    for (name, item) in rows {
        let desc = item.description.as_deref().unwrap_or("-");
        let items_str = if item.child_count > 0 {
            item.child_count.to_string()
        } else {
            "-".to_string()
        };
        println!(
            "{:<width_name$} {:<width_kind$} {:<width_desc$} {}",
            name,
            item.kind.as_str(),
            desc,
            items_str,
            width_name = max_name,
            width_kind = max_kind,
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
    wtr.write_record(["id", "name", "description", "kind", "path"])?;
    wtr.write_record([
        &item.id.to_string(),
        &item.name,
        item.description.as_deref().unwrap_or(""),
        item.kind.as_str(),
        &item.path.join("/"),
    ])?;
    wtr.flush()?;
    Ok(())
}

fn print_items_csv(items: &[ItemWithPath]) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(["id", "name", "description", "kind", "path"])?;
    for item in items {
        wtr.write_record([
            &item.id.to_string(),
            &item.name,
            item.description.as_deref().unwrap_or(""),
            item.kind.as_str(),
            &item.path.join("/"),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

fn print_list_items_csv(items: &[ListItem]) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(io::stdout());
    wtr.write_record(["id", "name", "description", "kind", "child_count"])?;
    for item in items {
        wtr.write_record([
            &item.id.to_string(),
            &item.name,
            item.description.as_deref().unwrap_or(""),
            item.kind.as_str(),
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

/// Tree rendering pieces, widened to a fixed four columns per level.
struct TreeChars {
    branch: String,
    last: String,
    vertical: String,
    space: &'static str,
}

impl TreeChars {
    fn current() -> Self {
        let t = glyph_set().tree();
        Self {
            branch: format!("{}{}{} ", t.tee, t.dash, t.dash),
            last: format!("{}{}{} ", t.elbow, t.dash, t.dash),
            vertical: format!("{}   ", t.pipe),
            space: "    ",
        }
    }
}

fn print_tree_items_human(items: &[TreeItem]) -> Result<()> {
    /// One level of the tree, with duplicates folded together.
    fn siblings(items: &[TreeItem]) -> Vec<(&TreeItem, usize)> {
        group_duplicates(items, |item| {
            (item.child_count == 0).then(|| sibling_key(&item.name, &item.description, item.kind))
        })
    }

    fn print_item_line(item: &TreeItem, count: usize) {
        if let Some(glyph) = item.kind.glyph() {
            print!("{} ", glyph);
        }
        print!("{}", counted_name(&item.name, count));
        if let Some(ref desc) = item.description {
            print!(" ({})", desc);
        }
        if item.child_count > 0 {
            print!(" [{}]", item.child_count);
        }
        println!();
    }

    fn print_subtree(
        item: &TreeItem,
        count: usize,
        prefix: &str,
        is_last: bool,
        chars: &TreeChars,
    ) {
        let connector = if is_last { &chars.last } else { &chars.branch };

        print!("{}{}", prefix, connector);
        print_item_line(item, count);

        let child_prefix = format!(
            "{}{}",
            prefix,
            if is_last {
                chars.space
            } else {
                &chars.vertical
            }
        );

        let children = siblings(&item.children);
        let rows = children.len();
        for (i, (child, count)) in children.into_iter().enumerate() {
            print_subtree(child, count, &child_prefix, i == rows - 1, chars);
        }
    }

    let chars = TreeChars::current();
    for (item, count) in siblings(items) {
        // Root items: print without prefix
        print_item_line(item, count);

        // Print children with tree structure
        let children = siblings(&item.children);
        let rows = children.len();
        for (i, (child, count)) in children.into_iter().enumerate() {
            print_subtree(child, count, "", i == rows - 1, &chars);
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
                kind: item.kind,
            });
            collect_flat(&item.children, result);
        }
    }

    let mut flat_items = Vec::new();
    collect_flat(items, &mut flat_items);
    print_list_items_csv(&flat_items)
}
