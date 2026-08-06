//! State and key handling for the interactive TUI.

use anyhow::{anyhow, Result};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::input::TextInput;
use crate::db;
use crate::model::Item;

/// A single visible row of the tree.
pub struct Node {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub depth: usize,
    pub child_count: usize,
    pub expanded: bool,
    /// One flag per ancestor level: true when that ancestor has later siblings.
    pub ancestors: Vec<bool>,
    pub is_last: bool,
    pub parent: Option<i64>,
    /// Place path with a trailing `/`, set only on filtered rows.
    /// Empty for a match that sits at root.
    pub place_path: Option<String>,
}

/// What the next keypress means.
pub enum Mode {
    Normal,
    Search,
    Prompt(Prompt),
    Confirm(Confirm),
}

pub struct Prompt {
    pub title: String,
    pub hint: String,
    pub input: TextInput,
    pub kind: PromptKind,
}

pub enum PromptKind {
    /// Add an item inside the given place, or at root when `None`.
    Add(Option<i64>),
    Rename(i64),
    Describe(i64),
    Move(i64),
}

pub struct Confirm {
    pub message: String,
    pub action: ConfirmAction,
}

pub enum ConfirmAction {
    Delete(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Error,
}

pub struct App {
    conn: Connection,
    items: HashMap<i64, Item>,
    pub nodes: Vec<Node>,
    expanded: HashSet<i64>,
    pub selected: usize,
    pub mode: Mode,
    pub filter: String,
    pub search_input: TextInput,
    pub status: Option<(String, StatusKind)>,
    pub show_help: bool,
    pub should_quit: bool,
    /// Rows the tree pane can show, updated on every draw.
    pub visible_rows: usize,
    /// Item to select once the next reload rebuilds the rows.
    pending_selection: Option<i64>,
}

impl App {
    pub fn new(db_path: Option<&Path>) -> Result<Self> {
        let conn = db::open(db_path)?;
        let mut app = Self {
            conn,
            items: HashMap::new(),
            nodes: Vec::new(),
            expanded: HashSet::new(),
            selected: 0,
            mode: Mode::Normal,
            filter: String::new(),
            search_input: TextInput::default(),
            status: None,
            show_help: false,
            should_quit: false,
            visible_rows: 10,
            pending_selection: None,
        };
        app.reload()?;
        // Start with the top level open so the tree is never an empty frame.
        app.expand_all();
        Ok(app)
    }

    /// Re-read every item from the database and rebuild the visible rows.
    fn reload(&mut self) -> Result<()> {
        let items = db::list_all_items(&self.conn)?;
        self.items = items.into_iter().map(|item| (item.id, item)).collect();
        self.rebuild_nodes();
        Ok(())
    }

    pub fn selected_node(&self) -> Option<&Node> {
        self.nodes.get(self.selected)
    }

    fn selected_id(&self) -> Option<i64> {
        self.selected_node().map(|node| node.id)
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Names from root down to `id`, inclusive.
    pub fn path_of(&self, id: i64) -> Vec<String> {
        let mut path = Vec::new();
        let mut current = Some(id);
        let mut guard = 0;

        while let Some(item_id) = current {
            let Some(item) = self.items.get(&item_id) else {
                break;
            };
            path.push(item.name.clone());
            current = item.place_id;
            guard += 1;
            if guard > self.items.len() {
                break;
            }
        }

        path.reverse();
        path
    }

    pub fn item(&self, id: i64) -> Option<&Item> {
        self.items.get(&id)
    }

    /// Parent id, ignoring references to rows that are no longer present.
    fn parent_of(&self, item: &Item) -> Option<i64> {
        item.place_id.filter(|id| self.items.contains_key(id))
    }

    fn children_map(&self) -> HashMap<Option<i64>, Vec<&Item>> {
        let mut map: HashMap<Option<i64>, Vec<&Item>> = HashMap::new();
        for item in self.items.values() {
            map.entry(self.parent_of(item)).or_default().push(item);
        }
        for children in map.values_mut() {
            children.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then(a.id.cmp(&b.id))
            });
        }
        map
    }

    fn rebuild_nodes(&mut self) {
        let previous = self.selected_id();

        self.nodes = if self.filter.is_empty() {
            self.build_tree()
        } else {
            self.build_filtered()
        };

        self.selected = previous
            .and_then(|id| self.nodes.iter().position(|node| node.id == id))
            .unwrap_or_else(|| self.selected.min(self.nodes.len().saturating_sub(1)));
    }

    fn build_tree(&self) -> Vec<Node> {
        let map = self.children_map();
        let mut nodes = Vec::new();
        let mut ancestors = Vec::new();
        push_level(&map, None, 0, &mut ancestors, &self.expanded, &mut nodes);
        nodes
    }

    fn build_filtered(&self) -> Vec<Node> {
        let needle = self.filter.to_lowercase();
        let map = self.children_map();

        let mut matches: Vec<&Item> = self
            .items
            .values()
            .filter(|item| {
                item.name.to_lowercase().contains(&needle)
                    || item
                        .description
                        .as_deref()
                        .is_some_and(|desc| desc.to_lowercase().contains(&needle))
            })
            .collect();

        matches.sort_by_cached_key(|item| {
            let path = self.path_of(item.id);
            path.join("/").to_lowercase()
        });

        matches
            .into_iter()
            .map(|item| {
                let path = self.path_of(item.id);
                // Always Some in filtered rows, so every match renders the same
                // way. Root items simply carry an empty prefix.
                let place_path = Some(if path.len() > 1 {
                    format!("{}/", path[..path.len() - 1].join("/"))
                } else {
                    String::new()
                });
                Node {
                    id: item.id,
                    name: item.name.clone(),
                    description: item.description.clone(),
                    depth: 0,
                    child_count: map.get(&Some(item.id)).map_or(0, |c| c.len()),
                    expanded: false,
                    ancestors: Vec::new(),
                    is_last: true,
                    parent: self.parent_of(item),
                    place_path,
                }
            })
            .collect()
    }

    fn select_id(&mut self, id: i64) {
        if let Some(index) = self.nodes.iter().position(|node| node.id == id) {
            self.selected = index;
        }
    }

    fn info(&mut self, message: impl Into<String>) {
        self.status = Some((message.into(), StatusKind::Info));
    }

    fn error(&mut self, message: impl Into<String>) {
        self.status = Some((message.into(), StatusKind::Error));
    }

    /// Run a database action, reporting either its message or its error.
    fn apply(&mut self, result: Result<String>) {
        match result {
            Ok(message) => {
                if let Err(err) = self.reload() {
                    self.error(format!("{err}"));
                } else {
                    self.info(message);
                }
            }
            Err(err) => self.error(format!("{err}")),
        }
    }

    // -- navigation ------------------------------------------------------

    fn move_selection(&mut self, delta: isize) {
        if self.nodes.is_empty() {
            return;
        }
        let last = self.nodes.len() as isize - 1;
        self.selected = (self.selected as isize + delta).clamp(0, last) as usize;
    }

    fn toggle(&mut self) {
        let Some(node) = self.selected_node() else {
            return;
        };
        if node.child_count == 0 {
            return;
        }
        let id = node.id;
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
        }
        self.rebuild_nodes();
    }

    fn expand(&mut self) {
        let Some(node) = self.selected_node() else {
            return;
        };
        if node.child_count == 0 {
            return;
        }
        if node.expanded {
            self.move_selection(1);
        } else {
            let id = node.id;
            self.expanded.insert(id);
            self.rebuild_nodes();
        }
    }

    fn collapse(&mut self) {
        let Some(node) = self.selected_node() else {
            return;
        };
        if node.expanded {
            let id = node.id;
            self.expanded.remove(&id);
            self.rebuild_nodes();
        } else if let Some(parent) = node.parent {
            self.select_id(parent);
        }
    }

    fn expand_all(&mut self) {
        let ids: Vec<i64> = self
            .items
            .values()
            .filter_map(|item| self.parent_of(item))
            .collect();
        self.expanded.extend(ids);
        self.rebuild_nodes();
    }

    fn collapse_all(&mut self) {
        self.expanded.clear();
        self.rebuild_nodes();
    }

    // -- actions ---------------------------------------------------------

    fn add_item(&mut self, place_id: Option<i64>, name: &str) -> Result<String> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("name cannot be empty"));
        }
        if name.contains('/') {
            return Err(anyhow!("name cannot contain '/'"));
        }
        if db::name_exists_in_place(&self.conn, name, place_id)? {
            return Err(anyhow!("'{name}' already exists here"));
        }

        let item = db::insert_item(&self.conn, name, None, place_id)?;
        if let Some(id) = place_id {
            self.expanded.insert(id);
        }
        self.pending_selection = Some(item.id);
        Ok(format!("Added '{name}'"))
    }

    fn rename_item(&mut self, id: i64, new_name: &str) -> Result<String> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err(anyhow!("name cannot be empty"));
        }
        if new_name.contains('/') {
            return Err(anyhow!("name cannot contain '/'"));
        }

        let item = self
            .items
            .get(&id)
            .ok_or_else(|| anyhow!("item no longer exists"))?;
        let old_name = item.name.clone();
        if old_name == new_name {
            return Ok(format!("'{old_name}' unchanged"));
        }

        let place_id = item.place_id;
        if db::name_exists_in_place(&self.conn, new_name, place_id)? {
            return Err(anyhow!("'{new_name}' already exists here"));
        }

        db::update_item_name(&self.conn, id, new_name)?;
        Ok(format!("Renamed '{old_name}' to '{new_name}'"))
    }

    fn describe_item(&mut self, id: i64, text: &str) -> Result<String> {
        let text = text.trim();
        let description = if text.is_empty() { None } else { Some(text) };
        db::update_item_description(&self.conn, id, description)?;

        let name = self
            .items
            .get(&id)
            .map_or("item", |item| item.name.as_str());
        Ok(match description {
            Some(_) => format!("Updated description of '{name}'"),
            None => format!("Cleared description of '{name}'"),
        })
    }

    fn move_item(&mut self, id: i64, destination: &str) -> Result<String> {
        let destination = destination.trim();
        let name = self
            .items
            .get(&id)
            .ok_or_else(|| anyhow!("item no longer exists"))?
            .name
            .clone();

        let new_place_id = if destination.is_empty() || destination == "/" || destination == "root"
        {
            None
        } else {
            let place = db::resolve_or_create_place(&self.conn, destination)?;
            if place.id == id || db::is_ancestor(&self.conn, id, place.id)? {
                return Err(anyhow!(
                    "cannot move '{name}' into itself or its descendants"
                ));
            }
            Some(place.id)
        };

        let current_place_id = self.items.get(&id).and_then(|item| item.place_id);
        if current_place_id != new_place_id
            && db::name_exists_in_place(&self.conn, &name, new_place_id)?
        {
            let target = if destination.is_empty() {
                "/"
            } else {
                destination
            };
            return Err(anyhow!("'{name}' already exists in {target}"));
        }

        db::move_item(&self.conn, id, new_place_id)?;
        if let Some(place_id) = new_place_id {
            self.expanded.insert(place_id);
        }
        self.pending_selection = Some(id);

        let target = if new_place_id.is_none() {
            "/".to_string()
        } else {
            destination.to_string()
        };
        Ok(format!("Moved '{name}' to {target}"))
    }

    fn delete_item(&mut self, id: i64) -> Result<String> {
        let name = self
            .items
            .get(&id)
            .ok_or_else(|| anyhow!("item no longer exists"))?
            .name
            .clone();
        db::delete_item(&self.conn, id)?;
        self.expanded.remove(&id);
        Ok(format!("Removed '{name}'"))
    }

    // -- key handling ----------------------------------------------------

    pub fn on_key(&mut self, key: KeyEvent) {
        if self.show_help {
            self.show_help = false;
            return;
        }

        match self.mode {
            Mode::Normal => self.on_key_normal(key),
            Mode::Search => self.on_key_search(key),
            Mode::Prompt(_) => self.on_key_prompt(key),
            Mode::Confirm(_) => self.on_key_confirm(key),
        }
    }

    fn on_key_normal(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let page = (self.visible_rows / 2).max(1) as isize;
        self.status = None;

        match key.code {
            KeyCode::Char('c') if ctrl => self.should_quit = true,
            KeyCode::Char('d') if ctrl => self.move_selection(page),
            KeyCode::Char('u') if ctrl => self.move_selection(-page),

            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => {
                if self.filter.is_empty() {
                    self.should_quit = true;
                } else {
                    self.clear_filter();
                }
            }

            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(page * 2),
            KeyCode::PageUp => self.move_selection(-page * 2),
            KeyCode::Char('g') | KeyCode::Home => self.selected = 0,
            KeyCode::Char('G') | KeyCode::End => self.selected = self.nodes.len().saturating_sub(1),

            KeyCode::Enter | KeyCode::Char(' ') => self.toggle(),
            KeyCode::Char('l') | KeyCode::Right => self.expand(),
            KeyCode::Char('h') | KeyCode::Left => self.collapse(),
            KeyCode::Char('E') => self.expand_all(),
            KeyCode::Char('C') => self.collapse_all(),

            KeyCode::Char('/') => {
                self.search_input = TextInput::new(self.filter.clone());
                self.mode = Mode::Search;
            }

            KeyCode::Char('a') => {
                let place = self.selected_id();
                let title = match place.map(|id| self.path_of(id).join("/")) {
                    Some(path) => format!("Add item in {path}"),
                    None => "Add item at root".to_string(),
                };
                self.open_prompt(title, "name", String::new(), PromptKind::Add(place));
            }
            KeyCode::Char('A') => {
                self.open_prompt(
                    "Add item at root".to_string(),
                    "name",
                    String::new(),
                    PromptKind::Add(None),
                );
            }
            KeyCode::Char('r') => {
                if let Some(node) = self.selected_node() {
                    let (id, name) = (node.id, node.name.clone());
                    self.open_prompt(
                        format!("Rename '{name}'"),
                        "new name",
                        name,
                        PromptKind::Rename(id),
                    );
                }
            }
            KeyCode::Char('d') => {
                if let Some(node) = self.selected_node() {
                    let id = node.id;
                    let name = node.name.clone();
                    let current = node.description.clone().unwrap_or_default();
                    self.open_prompt(
                        format!("Describe '{name}'"),
                        "description (empty clears)",
                        current,
                        PromptKind::Describe(id),
                    );
                }
            }
            KeyCode::Char('m') => {
                if let Some(node) = self.selected_node() {
                    let id = node.id;
                    let name = node.name.clone();
                    self.open_prompt(
                        format!("Move '{name}'"),
                        "destination path ('/' for root)",
                        String::new(),
                        PromptKind::Move(id),
                    );
                }
            }
            KeyCode::Char('x') | KeyCode::Delete => {
                if let Some(node) = self.selected_node() {
                    let message = if node.child_count > 0 {
                        format!(
                            "Remove '{}'? Its {} item(s) move to root.",
                            node.name, node.child_count
                        )
                    } else {
                        format!("Remove '{}'?", node.name)
                    };
                    self.mode = Mode::Confirm(Confirm {
                        message,
                        action: ConfirmAction::Delete(node.id),
                    });
                }
            }

            KeyCode::Char('R') => {
                let result = self.reload();
                match result {
                    Ok(()) => self.info("Reloaded"),
                    Err(err) => self.error(format!("{err}")),
                }
            }
            KeyCode::Char('?') => self.show_help = true,
            _ => {}
        }
    }

    fn open_prompt(&mut self, title: String, hint: &str, value: String, kind: PromptKind) {
        self.mode = Mode::Prompt(Prompt {
            title,
            hint: hint.to_string(),
            input: TextInput::new(value),
            kind,
        });
    }

    fn clear_filter(&mut self) {
        self.filter.clear();
        self.search_input.clear();
        self.rebuild_nodes();
    }

    fn on_key_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.clear_filter();
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                if self.nodes.is_empty() {
                    self.info(format!("No matches for '{}'", self.filter));
                }
            }
            KeyCode::Backspace => {
                self.search_input.backspace();
                self.sync_filter();
            }
            KeyCode::Delete => {
                self.search_input.delete();
                self.sync_filter();
            }
            KeyCode::Left => self.search_input.left(),
            KeyCode::Right => self.search_input.right(),
            KeyCode::Home => self.search_input.home(),
            KeyCode::End => self.search_input.end(),
            KeyCode::Char(c) => {
                self.search_input.insert(c);
                self.sync_filter();
            }
            _ => {}
        }
    }

    fn sync_filter(&mut self) {
        self.filter = self.search_input.value().to_string();
        self.selected = 0;
        self.rebuild_nodes();
        self.selected = 0;
    }

    fn on_key_prompt(&mut self, key: KeyEvent) {
        let Mode::Prompt(mut prompt) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };

        match key.code {
            KeyCode::Esc => {}
            KeyCode::Enter => {
                let value = prompt.input.value().to_string();
                let result = match prompt.kind {
                    PromptKind::Add(place) => self.add_item(place, &value),
                    PromptKind::Rename(id) => self.rename_item(id, &value),
                    PromptKind::Describe(id) => self.describe_item(id, &value),
                    PromptKind::Move(id) => self.move_item(id, &value),
                };
                self.apply(result);
                if let Some(id) = self.pending_selection.take() {
                    self.select_id(id);
                }
            }
            KeyCode::Backspace => {
                prompt.input.backspace();
                self.mode = Mode::Prompt(prompt);
            }
            KeyCode::Delete => {
                prompt.input.delete();
                self.mode = Mode::Prompt(prompt);
            }
            KeyCode::Left => {
                prompt.input.left();
                self.mode = Mode::Prompt(prompt);
            }
            KeyCode::Right => {
                prompt.input.right();
                self.mode = Mode::Prompt(prompt);
            }
            KeyCode::Home => {
                prompt.input.home();
                self.mode = Mode::Prompt(prompt);
            }
            KeyCode::End => {
                prompt.input.end();
                self.mode = Mode::Prompt(prompt);
            }
            KeyCode::Char(c) => {
                prompt.input.insert(c);
                self.mode = Mode::Prompt(prompt);
            }
            _ => self.mode = Mode::Prompt(prompt),
        }
    }

    fn on_key_confirm(&mut self, key: KeyEvent) {
        let Mode::Confirm(confirm) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let result = match confirm.action {
                    ConfirmAction::Delete(id) => self.delete_item(id),
                };
                self.apply(result);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => self.info("Cancelled"),
            _ => self.mode = Mode::Confirm(confirm),
        }
    }
}

/// Append one level of the tree, recursing into expanded places.
fn push_level(
    map: &HashMap<Option<i64>, Vec<&Item>>,
    parent: Option<i64>,
    depth: usize,
    ancestors: &mut Vec<bool>,
    expanded: &HashSet<i64>,
    out: &mut Vec<Node>,
) {
    let Some(children) = map.get(&parent) else {
        return;
    };

    for (index, item) in children.iter().enumerate() {
        let is_last = index + 1 == children.len();
        let child_count = map.get(&Some(item.id)).map_or(0, |c| c.len());
        let is_expanded = expanded.contains(&item.id) && child_count > 0;

        out.push(Node {
            id: item.id,
            name: item.name.clone(),
            description: item.description.clone(),
            depth,
            child_count,
            expanded: is_expanded,
            ancestors: ancestors.clone(),
            is_last,
            parent,
            place_path: None,
        });

        if is_expanded {
            ancestors.push(!is_last);
            push_level(map, Some(item.id), depth + 1, ancestors, expanded, out);
            ancestors.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEvent;
    use tempfile::TempDir;

    /// An app over a throwaway database, seeded with a small tree.
    fn app() -> (App, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("test.db");
        let mut app = App::new(Some(&path)).expect("open app");

        let garage = db::insert_item(&app.conn, "garage", None, None).unwrap();
        let toolbox = db::insert_item(&app.conn, "toolbox", None, Some(garage.id)).unwrap();
        db::insert_item(&app.conn, "hammer", Some("16oz claw"), Some(toolbox.id)).unwrap();
        db::insert_item(&app.conn, "bike", None, Some(garage.id)).unwrap();
        db::insert_item(&app.conn, "attic", None, None).unwrap();

        app.reload().unwrap();
        app.expand_all();
        (app, dir)
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    fn names(app: &App) -> Vec<&str> {
        app.nodes.iter().map(|node| node.name.as_str()).collect()
    }

    fn id_of(app: &App, name: &str) -> i64 {
        app.nodes
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("no row named '{name}'"))
            .id
    }

    #[test]
    fn tree_lists_children_under_their_place_in_name_order() {
        let (app, _dir) = app();
        assert_eq!(
            names(&app),
            ["attic", "garage", "bike", "toolbox", "hammer"]
        );
    }

    #[test]
    fn collapsing_hides_descendants() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "garage").unwrap();

        press(&mut app, KeyCode::Left);

        assert_eq!(names(&app), ["attic", "garage"]);
        assert!(!app.selected_node().unwrap().expanded);
    }

    #[test]
    fn collapsing_a_leaf_selects_its_place() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "hammer").unwrap();

        press(&mut app, KeyCode::Left);

        assert_eq!(app.selected_node().unwrap().name, "toolbox");
    }

    #[test]
    fn child_counts_ignore_collapsed_state() {
        let (mut app, _dir) = app();
        app.collapse_all();

        let garage = app.nodes.iter().find(|n| n.name == "garage").unwrap();
        assert_eq!(garage.child_count, 2);
        assert!(!garage.expanded);
    }

    #[test]
    fn adding_puts_the_item_inside_the_selection_and_selects_it() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "toolbox").unwrap();

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "wrench");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.selected_node().unwrap().name, "wrench");
        assert_eq!(
            app.path_of(id_of(&app, "wrench")),
            ["garage", "toolbox", "wrench"]
        );
    }

    #[test]
    fn adding_a_duplicate_name_reports_an_error() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "garage").unwrap();

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "bike");
        press(&mut app, KeyCode::Enter);

        let (message, kind) = app.status.as_ref().expect("a status");
        assert_eq!(*kind, StatusKind::Error);
        assert!(message.contains("already exists"), "{message}");
    }

    #[test]
    fn renaming_updates_the_row() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "bike").unwrap();

        press(&mut app, KeyCode::Char('r'));
        for _ in 0..4 {
            press(&mut app, KeyCode::Backspace);
        }
        type_text(&mut app, "e-bike");
        press(&mut app, KeyCode::Enter);

        assert!(names(&app).contains(&"e-bike"));
        assert!(!names(&app).contains(&"bike"));
    }

    #[test]
    fn describing_then_clearing_round_trips() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "bike").unwrap();

        press(&mut app, KeyCode::Char('d'));
        type_text(&mut app, "blue, needs a new chain");
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.selected_node().unwrap().description.as_deref(),
            Some("blue, needs a new chain")
        );

        press(&mut app, KeyCode::Char('d'));
        for _ in 0..40 {
            press(&mut app, KeyCode::Backspace);
        }
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.selected_node().unwrap().description, None);
    }

    #[test]
    fn escape_cancels_a_prompt_without_changing_anything() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "bike").unwrap();

        press(&mut app, KeyCode::Char('r'));
        type_text(&mut app, "xyz");
        press(&mut app, KeyCode::Esc);

        assert!(names(&app).contains(&"bike"));
        assert!(app.status.is_none());
    }

    #[test]
    fn moving_into_a_descendant_is_refused() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "garage").unwrap();

        press(&mut app, KeyCode::Char('m'));
        type_text(&mut app, "garage/toolbox");
        press(&mut app, KeyCode::Enter);

        let (message, kind) = app.status.as_ref().expect("a status");
        assert_eq!(*kind, StatusKind::Error);
        assert!(message.contains("descendants"), "{message}");
        assert_eq!(app.path_of(id_of(&app, "garage")), ["garage"]);
    }

    #[test]
    fn moving_to_root_reparents_the_item() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "hammer").unwrap();

        press(&mut app, KeyCode::Char('m'));
        type_text(&mut app, "/");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.path_of(id_of(&app, "hammer")), ["hammer"]);
    }

    #[test]
    fn removing_needs_confirmation() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "bike").unwrap();

        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Char('n'));
        assert!(names(&app).contains(&"bike"));

        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Char('y'));
        assert!(!names(&app).contains(&"bike"));
    }

    #[test]
    fn removing_a_place_leaves_its_children_at_root() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "toolbox").unwrap();

        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Char('y'));

        assert_eq!(app.path_of(id_of(&app, "hammer")), ["hammer"]);
    }

    #[test]
    fn search_matches_names_and_descriptions() {
        let (mut app, _dir) = app();

        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "16oz");
        assert_eq!(names(&app), ["hammer"]);

        for _ in 0..4 {
            press(&mut app, KeyCode::Backspace);
        }
        type_text(&mut app, "BIKE");
        assert_eq!(names(&app), ["bike"]);
    }

    #[test]
    fn search_rows_show_the_place_path() {
        let (mut app, _dir) = app();

        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "a");
        press(&mut app, KeyCode::Enter);

        let hammer = app.nodes.iter().find(|n| n.name == "hammer").unwrap();
        assert_eq!(hammer.place_path.as_deref(), Some("garage/toolbox/"));

        let attic = app.nodes.iter().find(|n| n.name == "attic").unwrap();
        assert_eq!(attic.place_path.as_deref(), Some(""));
    }

    #[test]
    fn escape_clears_the_search_before_it_quits() {
        let (mut app, _dir) = app();

        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "hammer");
        press(&mut app, KeyCode::Enter);

        press(&mut app, KeyCode::Esc);
        assert!(app.filter.is_empty());
        assert!(!app.should_quit);

        press(&mut app, KeyCode::Esc);
        assert!(app.should_quit);
    }

    #[test]
    fn help_opens_and_the_next_key_closes_it() {
        let (mut app, _dir) = app();

        press(&mut app, KeyCode::Char('?'));
        assert!(app.show_help);

        press(&mut app, KeyCode::Char('q'));
        assert!(!app.show_help);
        assert!(!app.should_quit);
    }
}
