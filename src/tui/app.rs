//! State and key handling for the interactive TUI.

use anyhow::{anyhow, Result};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::input::TextInput;
use crate::db;
use crate::inventory;
use crate::model::{glyph_set, group_duplicates, Item, Kind};
use crate::search;

/// A single visible row of the tree.
pub struct Node {
    /// The item the row acts on: the oldest of its duplicates.
    pub id: i64,
    /// Every item the row stands for, oldest first. More than one only for
    /// duplicates.
    pub ids: Vec<i64>,
    pub name: String,
    pub description: Option<String>,
    pub depth: usize,
    pub child_count: usize,
    pub kind: Kind,
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
    /// Add items to the given place, or to root when `None`. The prompt stays
    /// open for the next one, and counts how many it has added.
    Add {
        place: Option<i64>,
        added: usize,
    },
    Rename(i64),
    Describe(i64),
    /// Move these items to the place the user types.
    Move(Vec<i64>),
}

pub struct Confirm {
    pub message: String,
    pub action: ConfirmAction,
}

pub enum ConfirmAction {
    Delete(Vec<i64>),
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
    /// Items marked with Space or visual mode. `x`, `d` and `m` act on them.
    pub marks: HashSet<i64>,
    /// Items cut with `x`, waiting for `p` to put them somewhere.
    pub cut: HashSet<i64>,
    /// The row visual mode started on. While set, every row between it and
    /// the cursor counts as marked.
    pub visual_anchor: Option<usize>,
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
            marks: HashSet::new(),
            cut: HashSet::new(),
            visual_anchor: None,
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
        // Forget marks and cuts on items that are gone.
        self.marks.retain(|id| self.items.contains_key(id));
        self.cut.retain(|id| self.items.contains_key(id));
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
            .and_then(|id| self.position_of(id))
            .unwrap_or_else(|| self.selected.min(self.nodes.len().saturating_sub(1)));
    }

    fn build_tree(&self) -> Vec<Node> {
        let map = self.children_map();
        let mut nodes = Vec::new();
        let mut ancestors = Vec::new();
        self.push_level(&map, None, 0, &mut ancestors, &mut nodes);
        nodes
    }

    /// What decides whether two items share a row: being duplicates, and being
    /// marked and cut alike, so a row is never half marked. `None` for an item
    /// that holds something, which always has a row of its own.
    #[allow(clippy::type_complexity)]
    fn row_key(
        &self,
        item: &Item,
        child_count: usize,
    ) -> Option<((Option<i64>, String, Option<String>, Kind), bool, bool)> {
        (child_count == 0).then(|| {
            (
                item.duplicate_key(),
                self.marks.contains(&item.id),
                self.cut.contains(&item.id),
            )
        })
    }

    /// Append one level of the tree, recursing into expanded places.
    fn push_level(
        &self,
        map: &HashMap<Option<i64>, Vec<&Item>>,
        parent: Option<i64>,
        depth: usize,
        ancestors: &mut Vec<bool>,
        out: &mut Vec<Node>,
    ) {
        let Some(children) = map.get(&parent) else {
            return;
        };

        let child_count = |item: &Item| map.get(&Some(item.id)).map_or(0, |c| c.len());
        let rows = group_duplicates(children.iter().copied(), |item| {
            self.row_key(item, child_count(item))
        });

        for (index, group) in rows.iter().enumerate() {
            let item = group[0];
            let is_last = index + 1 == rows.len();
            let child_count = child_count(item);
            let is_expanded = self.expanded.contains(&item.id) && child_count > 0;

            out.push(Node {
                id: item.id,
                ids: group.iter().map(|item| item.id).collect(),
                name: item.name.clone(),
                description: item.description.clone(),
                kind: item.kind,
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
                self.push_level(map, Some(item.id), depth + 1, ancestors, out);
                ancestors.pop();
            }
        }
    }

    fn build_filtered(&self) -> Vec<Node> {
        let map = self.children_map();

        let matches = search::search(self.items.values(), &self.filter, |item| {
            self.path_of(item.id).join("/").to_lowercase()
        });
        let child_count = |item: &Item| map.get(&Some(item.id)).map_or(0, |c| c.len());
        let groups = group_duplicates(matches, |item| self.row_key(item, child_count(item)));

        groups
            .into_iter()
            .map(|group| {
                let item = group[0];
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
                    ids: group.iter().map(|item| item.id).collect(),
                    name: item.name.clone(),
                    description: item.description.clone(),
                    kind: item.kind,
                    depth: 0,
                    child_count: child_count(item),
                    expanded: false,
                    ancestors: Vec::new(),
                    is_last: true,
                    parent: self.parent_of(item),
                    place_path,
                }
            })
            .collect()
    }

    /// The row standing for `id`, which may be one of several duplicates.
    fn position_of(&self, id: i64) -> Option<usize> {
        self.nodes.iter().position(|node| node.ids.contains(&id))
    }

    fn select_id(&mut self, id: i64) {
        if let Some(index) = self.position_of(id) {
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
    ///
    /// On success the rows are rebuilt, and the cursor goes to the item the
    /// action asked for, if any.
    fn apply(&mut self, result: Result<String>) {
        match result {
            Ok(message) => {
                if let Err(err) = self.reload() {
                    self.error(format!("{err}"));
                } else {
                    self.info(message);
                }
                if let Some(id) = self.pending_selection.take() {
                    self.select_id(id);
                }
            }
            Err(err) => {
                self.pending_selection = None;
                self.error(format!("{err}"));
            }
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
        // New items start as things. `t` reclassifies them.
        let item = inventory::add(&self.conn, name, None, place_id, Kind::Thing)?;
        if let Some(id) = place_id {
            self.expanded.insert(id);
        }
        self.pending_selection = Some(item.id);
        Ok(format!("Added {}", self.counted(&item)?))
    }

    /// `'hdmi cable'`, or `'hdmi cable' ×3` when it has duplicates.
    fn counted(&self, item: &Item) -> Result<String> {
        let count = inventory::duplicate_count(&self.conn, item)?;
        Ok(if count > 1 {
            format!("'{}' {}{count}", item.name, glyph_set().times())
        } else {
            format!("'{}'", item.name)
        })
    }

    /// Add another of the cursor row's item.
    fn add_duplicate(&mut self) -> Result<String> {
        let node = self
            .selected_node()
            .ok_or_else(|| anyhow!("nothing selected"))?;
        if node.child_count > 0 {
            return Err(anyhow!(
                "only an item that holds nothing can have duplicates"
            ));
        }
        let item = self
            .items
            .get(&node.id)
            .ok_or_else(|| anyhow!("item no longer exists"))?;

        let new = inventory::add(
            &self.conn,
            &item.name,
            item.description.as_deref(),
            item.place_id,
            item.kind,
        )?;
        self.pending_selection = Some(new.id);
        Ok(format!("Added another: {}", self.counted(&new)?))
    }

    /// Remove one of the cursor row's duplicates, the newest. Removing the
    /// last one asks first, as `d` does.
    fn remove_duplicate(&mut self) {
        let Some(node) = self.selected_node() else {
            return;
        };
        if node.child_count > 0 {
            self.error("only an item that holds nothing can have duplicates");
            return;
        }
        let ids = node.ids.clone();
        let Some(&newest) = ids.last() else {
            return;
        };
        if ids.len() == 1 {
            self.mode = Mode::Confirm(Confirm {
                message: format!("Remove {}?", self.describe_ids(&ids)),
                action: ConfirmAction::Delete(ids),
            });
            return;
        }

        let name = node.name.clone();
        let result = db::delete_item(&self.conn, newest).map(|()| {
            self.pending_selection = Some(ids[0]);
            format!("Removed one '{name}', {} left", ids.len() - 1)
        });
        self.apply(result);
    }

    fn rename_item(&mut self, id: i64, new_name: &str) -> Result<String> {
        let item = self
            .items
            .get(&id)
            .ok_or_else(|| anyhow!("item no longer exists"))?;

        if inventory::rename(&self.conn, item, new_name)? {
            Ok(format!("Renamed '{}' to '{}'", item.name, new_name.trim()))
        } else {
            Ok(format!("'{}' unchanged", item.name))
        }
    }

    fn describe_item(&mut self, id: i64, text: &str) -> Result<String> {
        let set = inventory::describe(&self.conn, id, text)?;

        let name = self
            .items
            .get(&id)
            .map_or("item", |item| item.name.as_str());
        Ok(if set {
            format!("Updated description of '{name}'")
        } else {
            format!("Cleared description of '{name}'")
        })
    }

    /// Move items to the place typed as `destination`.
    fn move_items(&mut self, ids: &[i64], destination: &str) -> Result<String> {
        // A refused move must not leave an auto-created place behind.
        let tx = self.conn.unchecked_transaction()?;
        let place_id = inventory::resolve_destination(&tx, destination)?;
        self.move_all(&tx, ids, place_id)?;
        tx.commit()?;
        Ok(self.moved(ids, place_id))
    }

    /// Put the cut items into the selected item. A refusal keeps the cut, so
    /// it can go somewhere else.
    fn paste(&mut self) -> Result<String> {
        let place_id = self
            .selected_id()
            .ok_or_else(|| anyhow!("select a place to paste into"))?;
        let mut ids: Vec<i64> = self.cut.iter().copied().collect();
        ids.sort_unstable();

        let tx = self.conn.unchecked_transaction()?;
        self.move_all(&tx, &ids, Some(place_id))?;
        tx.commit()?;

        self.cut.clear();
        Ok(self.moved(&ids, Some(place_id)))
    }

    /// Move every item, or none of them: `conn` is a transaction.
    fn move_all(&self, conn: &Connection, ids: &[i64], place_id: Option<i64>) -> Result<()> {
        for id in ids {
            let item = self
                .items
                .get(id)
                .ok_or_else(|| anyhow!("item no longer exists"))?;
            inventory::move_to(conn, item, place_id)?;
        }
        Ok(())
    }

    /// Tidy up after a move, and say what moved where.
    fn moved(&mut self, ids: &[i64], place_id: Option<i64>) -> String {
        if let Some(place_id) = place_id {
            self.expanded.insert(place_id);
        }
        self.marks.clear();
        self.pending_selection = ids.first().copied();

        let target = place_id.map_or_else(|| "/".to_string(), |id| self.path_of(id).join("/"));
        format!("Moved {} to {target}", self.describe_ids(ids))
    }

    /// `'hammer'` for one item, `3 items` for several.
    fn describe_ids(&self, ids: &[i64]) -> String {
        match ids {
            [id] => format!(
                "'{}'",
                self.items.get(id).map_or("item", |i| i.name.as_str())
            ),
            _ => format!("{} items", ids.len()),
        }
    }

    /// Step the selection to the next or previous kind, and save it.
    fn cycle_kind(&mut self, forwards: bool) {
        let Some(node) = self.selected_node() else {
            return;
        };
        let (id, name) = (node.id, node.name.clone());
        let kind = if forwards {
            node.kind.next()
        } else {
            node.kind.previous()
        };

        let result =
            db::update_item_kind(&self.conn, id, kind).map(|()| format!("Set '{name}' to {kind}"));
        self.apply(result);
    }

    fn delete_items(&mut self, ids: &[i64]) -> Result<String> {
        let what = self.describe_ids(ids);
        let tx = self.conn.unchecked_transaction()?;
        for id in ids {
            db::delete_item(&tx, *id)?;
        }
        tx.commit()?;

        for id in ids {
            self.expanded.remove(id);
        }
        self.marks.clear();
        Ok(format!("Removed {what}"))
    }

    // -- marks -----------------------------------------------------------

    /// What `x`, `d` and `m` act on: every marked item, or else one item from
    /// the cursor row, the oldest of its duplicates.
    fn targets(&self) -> Vec<i64> {
        if self.marks.is_empty() {
            return self.selected_id().into_iter().collect();
        }
        let mut ids: Vec<i64> = self.marks.iter().copied().collect();
        ids.sort_unstable();
        ids
    }

    /// Whether the row at `index` is marked, counting a visual range.
    pub fn is_marked(&self, index: usize) -> bool {
        if let Some(anchor) = self.visual_anchor {
            if (anchor.min(self.selected)..=anchor.max(self.selected)).contains(&index) {
                return true;
            }
        }
        self.nodes
            .get(index)
            .is_some_and(|node| node.ids.iter().all(|id| self.marks.contains(id)))
    }

    /// Whether the row's items are cut, waiting for `p`.
    pub fn is_cut(&self, node: &Node) -> bool {
        node.ids.iter().all(|id| self.cut.contains(id))
    }

    /// Mark the cursor row, every duplicate on it, or unmark it if marked.
    fn toggle_mark(&mut self) {
        let Some(node) = self.selected_node() else {
            return;
        };
        let ids = node.ids.clone();
        if ids.iter().all(|id| self.marks.contains(id)) {
            for id in &ids {
                self.marks.remove(id);
            }
        } else {
            self.marks.extend(ids);
        }
        self.rebuild_nodes();
    }

    /// Leave visual mode, keeping its range marked.
    fn end_visual(&mut self) {
        let Some(anchor) = self.visual_anchor.take() else {
            return;
        };
        let (low, high) = (anchor.min(self.selected), anchor.max(self.selected));
        let ids: Vec<i64> = self
            .nodes
            .iter()
            .take(high + 1)
            .skip(low)
            .flat_map(|node| node.ids.iter().copied())
            .collect();
        self.marks.extend(ids);
        self.rebuild_nodes();
    }

    /// Ask before removing the targets.
    fn confirm_delete(&mut self) {
        let ids = self.targets();
        if ids.is_empty() {
            return;
        }
        // Contents that are not removed too go to root.
        let inside = self
            .items
            .values()
            .filter(|item| {
                item.place_id.is_some_and(|p| ids.contains(&p)) && !ids.contains(&item.id)
            })
            .count();
        let what = self.describe_ids(&ids);
        let message = if inside > 0 {
            format!("Remove {what}? {inside} item(s) inside move to root.")
        } else {
            format!("Remove {what}?")
        };
        self.mode = Mode::Confirm(Confirm {
            message,
            action: ConfirmAction::Delete(ids),
        });
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

        // Visual mode lasts while the cursor moves. Any other key keeps the
        // range marked and leaves it, and then does its usual job.
        let moves = matches!(
            key.code,
            KeyCode::Char('j' | 'k' | 'g' | 'G')
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
        ) || (ctrl && matches!(key.code, KeyCode::Char('d' | 'u')));
        if self.visual_anchor.is_some() && !moves {
            self.end_visual();
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('v')) {
                return;
            }
        }

        match key.code {
            KeyCode::Char('c') if ctrl => self.should_quit = true,
            KeyCode::Char('d') if ctrl => self.move_selection(page),
            KeyCode::Char('u') if ctrl => self.move_selection(-page),

            KeyCode::Char('q') => self.should_quit = true,
            // Clears one thing per press, and never quits.
            KeyCode::Esc => {
                if !self.marks.is_empty() {
                    self.marks.clear();
                    self.rebuild_nodes();
                } else if !self.filter.is_empty() {
                    self.clear_filter();
                }
            }

            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(page * 2),
            KeyCode::PageUp => self.move_selection(-page * 2),
            KeyCode::Char('g') | KeyCode::Home => self.selected = 0,
            KeyCode::Char('G') | KeyCode::End => self.selected = self.nodes.len().saturating_sub(1),

            KeyCode::Enter => self.toggle(),
            KeyCode::Char('l') | KeyCode::Right => self.expand(),
            KeyCode::Char('h') | KeyCode::Left => self.collapse(),
            KeyCode::Char('E') => self.expand_all(),
            KeyCode::Char('C') => self.collapse_all(),

            KeyCode::Char(' ') => {
                self.toggle_mark();
                self.move_selection(1);
            }
            KeyCode::Char('v') => {
                if !self.nodes.is_empty() {
                    self.visual_anchor = Some(self.selected);
                }
            }

            KeyCode::Char('/') => {
                self.search_input = TextInput::new(self.filter.clone());
                self.mode = Mode::Search;
            }

            // Beside the cursor row, inside it, or at root.
            KeyCode::Char('a') => {
                let place = self.selected_node().and_then(|node| node.parent);
                self.open_add(place);
            }
            KeyCode::Char('i') => self.open_add(self.selected_id()),
            KeyCode::Char('A') => self.open_add(None),
            KeyCode::Char('+') => {
                let result = self.add_duplicate();
                self.apply(result);
            }
            KeyCode::Char('-') => self.remove_duplicate(),
            KeyCode::Char('r') => {
                if let Some(node) = self.selected_node() {
                    let (id, name) = (node.id, node.name.clone());
                    self.open_prompt(
                        format!("Rename '{name}'"),
                        "new name  —  ⏎ confirm, Esc cancel",
                        name,
                        PromptKind::Rename(id),
                    );
                }
            }
            KeyCode::Char('e') => {
                if let Some(node) = self.selected_node() {
                    let id = node.id;
                    let name = node.name.clone();
                    let current = node.description.clone().unwrap_or_default();
                    self.open_prompt(
                        format!("Describe '{name}'"),
                        "description, empty clears  —  ⏎ confirm, Esc cancel",
                        current,
                        PromptKind::Describe(id),
                    );
                }
            }
            KeyCode::Char('m') => {
                let ids = self.targets();
                if !ids.is_empty() {
                    let title = format!("Move {}", self.describe_ids(&ids));
                    self.open_prompt(
                        title,
                        "destination path, / for root  —  ⏎ confirm, Esc cancel",
                        String::new(),
                        PromptKind::Move(ids),
                    );
                }
            }
            KeyCode::Char('x') => {
                let ids = self.targets();
                if !ids.is_empty() {
                    let what = self.describe_ids(&ids);
                    self.cut = ids.into_iter().collect();
                    self.marks.clear();
                    self.rebuild_nodes();
                    self.info(format!(
                        "Cut {what}. Select a place and press p, or X to cancel"
                    ));
                }
            }
            KeyCode::Char('X') => {
                if !self.cut.is_empty() {
                    self.cut.clear();
                    self.rebuild_nodes();
                    self.info("Cut cancelled");
                }
            }
            KeyCode::Char('p') => {
                if self.cut.is_empty() {
                    self.info("Nothing cut. Press x on an item first");
                } else {
                    let result = self.paste();
                    self.apply(result);
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => self.confirm_delete(),

            // 'k' is already "move up", so kind cycles on 't' for type.
            KeyCode::Char('t') => self.cycle_kind(true),
            KeyCode::Char('T') => self.cycle_kind(false),

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

    fn open_add(&mut self, place: Option<i64>) {
        let title = self.add_title(place, 0);
        self.open_prompt(
            title,
            "name  —  ⏎ add, empty line or Esc to finish",
            String::new(),
            PromptKind::Add { place, added: 0 },
        );
    }

    fn add_title(&self, place: Option<i64>, added: usize) -> String {
        let place = match place {
            Some(id) => format!("Add in {}", self.path_of(id).join("/")),
            None => "Add at root".to_string(),
        };
        if added == 0 {
            place
        } else {
            format!("{place} ({added} added)")
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
                if let PromptKind::Add { place, added } = prompt.kind {
                    // An empty line finishes. Otherwise add, and stay open for
                    // the next name, keeping what was typed if it was refused.
                    if value.trim().is_empty() {
                        return;
                    }
                    let result = self.add_item(place, &value);
                    if result.is_ok() {
                        prompt.input = TextInput::default();
                        prompt.kind = PromptKind::Add {
                            place,
                            added: added + 1,
                        };
                        prompt.title = self.add_title(place, added + 1);
                    }
                    self.apply(result);
                    self.mode = Mode::Prompt(prompt);
                    return;
                }
                let result = match prompt.kind {
                    PromptKind::Add { .. } => unreachable!("handled above"),
                    PromptKind::Rename(id) => self.rename_item(id, &value),
                    PromptKind::Describe(id) => self.describe_item(id, &value),
                    PromptKind::Move(ids) => self.move_items(&ids, &value),
                };
                self.apply(result);
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
                    ConfirmAction::Delete(ids) => self.delete_items(&ids),
                };
                self.apply(result);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => self.info("Cancelled"),
            _ => self.mode = Mode::Confirm(confirm),
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

        let garage = db::insert_item(&app.conn, "garage", None, None, Kind::Room).unwrap();
        let toolbox =
            db::insert_item(&app.conn, "toolbox", None, Some(garage.id), Kind::Box).unwrap();
        db::insert_item(
            &app.conn,
            "hammer",
            Some("16oz claw"),
            Some(toolbox.id),
            Kind::Thing,
        )
        .unwrap();
        db::insert_item(&app.conn, "bike", None, Some(garage.id), Kind::Thing).unwrap();
        db::insert_item(&app.conn, "attic", None, None, Kind::Room).unwrap();

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
    fn i_puts_the_item_inside_the_selection_and_selects_it() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "toolbox").unwrap();

        press(&mut app, KeyCode::Char('i'));
        type_text(&mut app, "wrench");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.selected_node().unwrap().name, "wrench");
        assert_eq!(
            app.path_of(id_of(&app, "wrench")),
            ["garage", "toolbox", "wrench"]
        );
    }

    #[test]
    fn adding_a_name_already_there_adds_a_duplicate() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "garage").unwrap();

        press(&mut app, KeyCode::Char('i'));
        type_text(&mut app, "bike");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.status.as_ref().expect("a status").1, StatusKind::Info);
        let bikes: Vec<&Node> = app.nodes.iter().filter(|n| n.name == "bike").collect();
        assert_eq!(bikes.len(), 1);
        assert_eq!(bikes[0].ids.len(), 2);
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

        press(&mut app, KeyCode::Char('e'));
        type_text(&mut app, "blue, needs a new chain");
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.selected_node().unwrap().description.as_deref(),
            Some("blue, needs a new chain")
        );

        press(&mut app, KeyCode::Char('e'));
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
    fn a_refused_move_creates_no_place() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "garage").unwrap();

        press(&mut app, KeyCode::Char('m'));
        type_text(&mut app, "garage/shelf");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.status.as_ref().expect("a status").1, StatusKind::Error);
        assert!(db::find_items_by_path(&app.conn, "garage/shelf")
            .unwrap()
            .is_empty());
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

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('n'));
        assert!(names(&app).contains(&"bike"));

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));
        assert!(!names(&app).contains(&"bike"));
    }

    #[test]
    fn removing_a_place_leaves_its_children_at_root() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "toolbox").unwrap();

        press(&mut app, KeyCode::Char('d'));
        press(&mut app, KeyCode::Char('y'));

        assert_eq!(app.path_of(id_of(&app, "hammer")), ["hammer"]);
    }

    #[test]
    fn t_cycles_the_kind_forwards_and_saves_it() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "bike").unwrap();
        assert_eq!(app.selected_node().unwrap().kind, Kind::Thing);

        press(&mut app, KeyCode::Char('t'));
        assert_eq!(app.selected_node().unwrap().kind, Kind::Room);

        // The change reached the database, not just the node.
        let id = id_of(&app, "bike");
        assert_eq!(
            db::get_item_by_id(&app.conn, id).unwrap().unwrap().kind,
            Kind::Room
        );
    }

    #[test]
    fn shift_t_cycles_the_kind_backwards() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "bike").unwrap();

        press(&mut app, KeyCode::Char('T'));
        assert_eq!(app.selected_node().unwrap().kind, Kind::Box);
    }

    #[test]
    fn cycling_the_kind_keeps_the_selection() {
        let (mut app, _dir) = app();
        app.selected = app.nodes.iter().position(|n| n.name == "hammer").unwrap();

        press(&mut app, KeyCode::Char('t'));

        assert_eq!(app.selected_node().unwrap().name, "hammer");
    }

    /// Put `count` duplicates called `name` in the garage, and reload.
    fn add_duplicates(app: &mut App, name: &str, count: usize) -> Vec<i64> {
        let garage = id_of(app, "garage");
        let ids = (0..count)
            .map(|_| {
                db::insert_item(&app.conn, name, None, Some(garage), Kind::Thing)
                    .unwrap()
                    .id
            })
            .collect();
        app.reload().unwrap();
        ids
    }

    #[test]
    fn duplicates_share_one_row() {
        let (mut app, _dir) = app();
        let ids = add_duplicates(&mut app, "hdmi cable", 3);

        let rows: Vec<&Node> = app
            .nodes
            .iter()
            .filter(|n| n.name == "hdmi cable")
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ids, ids);
        assert_eq!(rows[0].id, ids[0], "the oldest stands for the row");
    }

    #[test]
    fn a_duplicate_that_differs_gets_its_own_row() {
        let (mut app, _dir) = app();
        add_duplicates(&mut app, "hdmi cable", 2);
        let garage = id_of(&app, "garage");
        db::insert_item(
            &app.conn,
            "hdmi cable",
            Some("2m"),
            Some(garage),
            Kind::Thing,
        )
        .unwrap();
        app.reload().unwrap();

        let counts: Vec<usize> = app
            .nodes
            .iter()
            .filter(|n| n.name == "hdmi cable")
            .map(|n| n.ids.len())
            .collect();
        assert_eq!(counts, [2, 1]);
    }

    #[test]
    fn describing_one_duplicate_splits_it_out_and_keeps_it_selected() {
        let (mut app, _dir) = app();
        let ids = add_duplicates(&mut app, "hdmi cable", 3);
        app.select_id(ids[0]);

        press(&mut app, KeyCode::Char('e'));
        type_text(&mut app, "2m");
        press(&mut app, KeyCode::Enter);

        let selected = app.selected_node().unwrap();
        assert_eq!(selected.description.as_deref(), Some("2m"));
        assert_eq!(selected.ids, [ids[0]]);
        let rest = app
            .nodes
            .iter()
            .find(|n| n.name == "hdmi cable" && n.description.is_none())
            .unwrap();
        assert_eq!(rest.ids, ids[1..]);
    }

    #[test]
    fn search_groups_duplicates() {
        let (mut app, _dir) = app();
        add_duplicates(&mut app, "hdmi cable", 2);

        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "hdmi");

        assert_eq!(names(&app), ["hdmi cable"]);
        assert_eq!(app.nodes[0].ids.len(), 2);
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
    fn search_matches_words_in_any_order() {
        let (mut app, _dir) = app();

        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "claw 16");

        assert_eq!(names(&app), ["hammer"]);
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
    fn escape_clears_the_search_and_never_quits() {
        let (mut app, _dir) = app();

        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "hammer");
        press(&mut app, KeyCode::Enter);

        press(&mut app, KeyCode::Esc);
        assert!(app.filter.is_empty());

        press(&mut app, KeyCode::Esc);
        assert!(!app.should_quit);
    }

    fn select(app: &mut App, name: &str) {
        app.selected = app
            .nodes
            .iter()
            .position(|n| n.name == name)
            .unwrap_or_else(|| panic!("no row named '{name}'"));
    }

    fn mark(app: &mut App, name: &str) {
        select(app, name);
        press(app, KeyCode::Char(' '));
    }

    #[test]
    fn space_marks_the_row_and_moves_down() {
        let (mut app, _dir) = app();

        mark(&mut app, "bike");

        assert!(app.marks.contains(&id_of(&app, "bike")));
        assert_eq!(app.selected_node().unwrap().name, "toolbox");
    }

    #[test]
    fn space_on_a_marked_row_unmarks_it() {
        let (mut app, _dir) = app();

        mark(&mut app, "bike");
        mark(&mut app, "bike");

        assert!(app.marks.is_empty());
    }

    #[test]
    fn marking_a_duplicates_row_marks_every_duplicate() {
        let (mut app, _dir) = app();
        let ids = add_duplicates(&mut app, "hdmi cable", 3);
        app.select_id(ids[0]);

        press(&mut app, KeyCode::Char(' '));

        assert_eq!(app.marks, ids.into_iter().collect());
    }

    #[test]
    fn visual_mode_marks_the_range_and_keeps_it_on_escape() {
        let (mut app, _dir) = app();
        select(&mut app, "attic");

        press(&mut app, KeyCode::Char('v'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Char('j'));
        press(&mut app, KeyCode::Esc);

        assert!(app.visual_anchor.is_none());
        let expected = ["attic", "garage", "bike"].map(|name| id_of(&app, name));
        assert_eq!(app.marks, expected.into_iter().collect());
    }

    #[test]
    fn marks_survive_a_new_search() {
        let (mut app, _dir) = app();
        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "bike");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char(' '));

        press(&mut app, KeyCode::Char('/'));
        for _ in 0..4 {
            press(&mut app, KeyCode::Backspace);
        }
        type_text(&mut app, "hammer");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char(' '));

        assert_eq!(app.marks.len(), 2);
    }

    #[test]
    fn cutting_and_pasting_moves_the_marked_items_into_the_selected_place() {
        let (mut app, _dir) = app();
        mark(&mut app, "bike");
        mark(&mut app, "hammer");

        press(&mut app, KeyCode::Char('x'));
        assert!(app.marks.is_empty());
        select(&mut app, "attic");
        press(&mut app, KeyCode::Char('p'));

        assert_eq!(app.path_of(id_of(&app, "bike")), ["attic", "bike"]);
        assert_eq!(app.path_of(id_of(&app, "hammer")), ["attic", "hammer"]);
        assert!(app.cut.is_empty());
    }

    #[test]
    fn cutting_without_marks_takes_one_duplicate() {
        let (mut app, _dir) = app();
        let ids = add_duplicates(&mut app, "hdmi cable", 3);
        app.select_id(ids[0]);

        press(&mut app, KeyCode::Char('x'));
        select(&mut app, "attic");
        press(&mut app, KeyCode::Char('p'));

        let attic = id_of(&app, "attic");
        let moved = ids
            .iter()
            .filter(|id| app.item(**id).unwrap().place_id == Some(attic))
            .count();
        assert_eq!(moved, 1);
    }

    #[test]
    fn a_cut_duplicate_shows_on_its_own_row() {
        let (mut app, _dir) = app();
        let ids = add_duplicates(&mut app, "hdmi cable", 3);
        app.select_id(ids[0]);

        press(&mut app, KeyCode::Char('x'));

        let rows: Vec<&Vec<i64>> = app
            .nodes
            .iter()
            .filter(|n| n.name == "hdmi cable")
            .map(|n| &n.ids)
            .collect();
        assert_eq!(rows, [&vec![ids[0]], &ids[1..].to_vec()]);
    }

    #[test]
    fn pasting_with_nothing_cut_says_so() {
        let (mut app, _dir) = app();
        select(&mut app, "attic");

        press(&mut app, KeyCode::Char('p'));

        let (message, kind) = app.status.as_ref().expect("a status");
        assert_eq!(*kind, StatusKind::Info);
        assert!(message.contains("Nothing"), "{message}");
    }

    #[test]
    fn capital_x_cancels_the_cut() {
        let (mut app, _dir) = app();
        select(&mut app, "bike");

        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Char('X'));

        assert!(app.cut.is_empty());
    }

    #[test]
    fn pasting_a_place_into_its_own_contents_moves_nothing() {
        let (mut app, _dir) = app();
        select(&mut app, "garage");
        press(&mut app, KeyCode::Char('x'));

        select(&mut app, "toolbox");
        press(&mut app, KeyCode::Char('p'));

        assert_eq!(app.status.as_ref().expect("a status").1, StatusKind::Error);
        assert_eq!(app.path_of(id_of(&app, "garage")), ["garage"]);
        assert!(!app.cut.is_empty(), "the cut is kept to paste elsewhere");
    }

    #[test]
    fn d_removes_every_marked_item_after_confirming() {
        let (mut app, _dir) = app();
        mark(&mut app, "bike");
        mark(&mut app, "attic");

        press(&mut app, KeyCode::Char('d'));
        let Mode::Confirm(confirm) = &app.mode else {
            panic!("no confirmation");
        };
        assert!(confirm.message.contains("2 items"), "{}", confirm.message);
        press(&mut app, KeyCode::Char('y'));

        assert!(!names(&app).contains(&"bike"));
        assert!(!names(&app).contains(&"attic"));
        assert!(app.marks.is_empty());
    }

    #[test]
    fn m_moves_every_marked_item() {
        let (mut app, _dir) = app();
        mark(&mut app, "bike");
        mark(&mut app, "hammer");

        press(&mut app, KeyCode::Char('m'));
        type_text(&mut app, "attic");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.path_of(id_of(&app, "bike")), ["attic", "bike"]);
        assert_eq!(app.path_of(id_of(&app, "hammer")), ["attic", "hammer"]);
    }

    #[test]
    fn escape_leaves_visual_mode_then_clears_marks_then_the_search() {
        let (mut app, _dir) = app();
        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "garage");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('v'));

        press(&mut app, KeyCode::Esc);
        assert!(app.visual_anchor.is_none());
        assert!(!app.marks.is_empty());

        press(&mut app, KeyCode::Esc);
        assert!(app.marks.is_empty());
        assert!(!app.filter.is_empty());

        press(&mut app, KeyCode::Esc);
        assert!(app.filter.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn enter_folds_a_place() {
        let (mut app, _dir) = app();
        select(&mut app, "garage");

        press(&mut app, KeyCode::Enter);

        assert_eq!(names(&app), ["attic", "garage"]);
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

    #[test]
    fn a_adds_beside_the_cursor_row() {
        let (mut app, _dir) = app();
        select(&mut app, "bike");

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "saw");
        press(&mut app, KeyCode::Enter);

        assert_eq!(app.path_of(id_of(&app, "saw")), ["garage", "saw"]);
    }

    #[test]
    fn the_add_prompt_stays_open_until_an_empty_line() {
        let (mut app, _dir) = app();
        select(&mut app, "toolbox");

        press(&mut app, KeyCode::Char('i'));
        type_text(&mut app, "wrench");
        press(&mut app, KeyCode::Enter);
        type_text(&mut app, "pliers");
        press(&mut app, KeyCode::Enter);

        let Mode::Prompt(prompt) = &app.mode else {
            panic!("the prompt closed");
        };
        assert!(prompt.title.contains("2 added"), "{}", prompt.title);
        assert_eq!(prompt.input.value(), "");

        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Normal));
        assert_eq!(app.selected_node().unwrap().name, "pliers");
        assert_eq!(
            app.path_of(id_of(&app, "wrench")),
            ["garage", "toolbox", "wrench"]
        );
    }

    #[test]
    fn a_refused_name_keeps_the_prompt_and_what_was_typed() {
        let (mut app, _dir) = app();
        select(&mut app, "toolbox");

        press(&mut app, KeyCode::Char('i'));
        type_text(&mut app, "a/b");
        press(&mut app, KeyCode::Enter);

        let Mode::Prompt(prompt) = &app.mode else {
            panic!("the prompt closed");
        };
        assert_eq!(prompt.input.value(), "a/b");
        assert_eq!(app.status.as_ref().expect("a status").1, StatusKind::Error);
    }

    #[test]
    fn adding_a_duplicate_says_how_many_there_are() {
        let (mut app, _dir) = app();
        select(&mut app, "bike");

        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "bike");
        press(&mut app, KeyCode::Enter);

        let (message, _) = app.status.as_ref().expect("a status");
        assert!(message.contains("×2"), "{message}");
    }

    #[test]
    fn plus_adds_a_duplicate_of_the_row() {
        let (mut app, _dir) = app();
        select(&mut app, "bike");

        press(&mut app, KeyCode::Char('+'));

        let row = app.selected_node().unwrap();
        assert_eq!(row.name, "bike");
        assert_eq!(row.ids.len(), 2);
    }

    #[test]
    fn plus_on_a_place_is_refused() {
        let (mut app, _dir) = app();
        select(&mut app, "toolbox");

        press(&mut app, KeyCode::Char('+'));

        assert_eq!(app.status.as_ref().expect("a status").1, StatusKind::Error);
        assert_eq!(app.item_count(), 5);
    }

    #[test]
    fn minus_removes_one_duplicate_without_asking() {
        let (mut app, _dir) = app();
        select(&mut app, "bike");
        press(&mut app, KeyCode::Char('+'));

        press(&mut app, KeyCode::Char('-'));

        assert!(matches!(app.mode, Mode::Normal));
        assert_eq!(app.selected_node().unwrap().ids.len(), 1);
    }

    #[test]
    fn minus_on_the_last_one_asks_first() {
        let (mut app, _dir) = app();
        select(&mut app, "bike");

        press(&mut app, KeyCode::Char('-'));
        assert!(matches!(app.mode, Mode::Confirm(_)));
        press(&mut app, KeyCode::Char('y'));

        assert!(!names(&app).contains(&"bike"));
    }
}
