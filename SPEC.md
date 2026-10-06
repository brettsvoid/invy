# invy - Specification

A command-line tool for tracking home inventory with hierarchical places.

## Core Concepts

### Items
Everything in invy is an **item**. An item has:
- **name** (required): what the item is called. Names can repeat, even in one place
- **description** (optional): free-form text
- **place** (optional): parent item that holds this item

### Places
A place is just an item that contains other items. There's no distinction between "item" and "place" - any item can hold other items.

### Kinds

Every item has a kind. The set is fixed, so a typo is a parse error rather than
a fourth kind. The first three describe a place. `thing` is the default.

| Kind | Nerd Font (default) | Unicode | Covers |
|------|-----------|---------|--------|
| `room` | `md-home_variant` U+F02DE | `⌂` | A room, a loft, a shed, a garden |
| `furniture` | `md-dresser` U+F0F4A | `▤` | A cupboard, a dresser, a shelf, a workbench |
| `box` | `md-package_variant_closed` U+F03D7 | `▣` | A box, a bag, a case, a toolbox |
| `thing` | none | none | Anything you put in a place |

The glyph set is chosen with `--glyphs`. It also decides the fold markers and
the tree branches:

| Set | Expanded | Collapsed | Leaf | Branches | Duplicates |
|-----|----------|-----------|------|----------|------------|
| `nerd` (default) | `cod-triangle_down` U+EB6E | `cod-triangle_right` U+EB70 | `cod-circle_small` U+EC07 | `├── └── │` | `×` |
| `unicode` | `▾` | `▸` | `·` | `├── └── │` | `×` |
| `ascii` | `v` | `>` | `-` | `\|--` `` `-- `` `\|` | `x` |

Every glyph is one column wide, so the tree lines up in all three sets. `nerd`
is the default. Its codepoints sit in the Private Use Area, so a terminal
without a patched font draws blank boxes and should use `unicode` instead.
`ascii` emits no character above U+007F, which makes it the one to pipe.

A kind is descriptive, not structural. It does not restrict what can go where,
and nothing else in `invy` reads it. A place auto-created on the way to an item
starts as a `thing`, because `invy` cannot know what sort of place it is.

### Hierarchy
Items form a tree structure:
```
(root)
├── garage
│   └── toolbox
│       ├── hammer
│       └── screwdriver
└── kitchen
    └── drawer
        └── scissors
```

### Duplicates
An item is one physical thing, so three identical cables are three items.
Items are **duplicates** when they sit in the same place with the same name,
description and kind, and none of them holds anything. Duplicates are
interchangeable. See `docs/adr/0001-no-item-quantities.md`.

Human output shows duplicates as one entry with a count, as in
`hdmi cable ×3`. `--json` and `--csv` never group: each item is its own
object or row, with its own id.

### References
Every command that takes an item or a place takes a reference:

| Reference | Matches |
|-----------|---------|
| `hammer` | Every item called `hammer`, anywhere |
| `toolbox/hammer` | Every `hammer` directly in a `toolbox` at that path |
| `@14` | The item with id 14, and nothing else |

Names match without regard to ASCII case. When a reference matches
duplicates, any one of them will do, and the command acts on the oldest. When
it matches items that are not duplicates, it is ambiguous, and the error lists
each match by `@id`:

```
Error: 'pi power supply' is ambiguous. Use a path or an @id:
  @14  office/pi power supply  5V 3A
  @27  office/pi power supply  5V 5A
```

An item's id appears in `--json` and `--csv` output.

---

## Global Flags

All commands support these flags:

| Flag | Short | Description |
|------|-------|-------------|
| `--json` | `-j` | Output as JSON |
| `--csv` | | Output as CSV |
| `--db <path>` | | Use custom database file |
| `--glyphs <set>` | | Tree characters: `nerd` (default), `unicode`, `ascii`. Reads `INVY_GLYPHS` |

**Default database location:** `~/.local/share/invy/invy.db`, or
`$XDG_DATA_HOME/invy/invy.db` when `XDG_DATA_HOME` is set, unless the config
file names another. invy creates the `invy` directory if it is missing. It
does not create the directory of a path given by `--db` or the config file.

`--glyphs` on the command line beats `INVY_GLYPHS` in the environment.

---

## Configuration

invy reads `$XDG_CONFIG_HOME/invy/config.toml`, or `~/.config/invy/config.toml`
when `XDG_CONFIG_HOME` is not set. This is the same path on every platform,
macOS included. The file is optional, and invy does not create it.

```toml
# Database file. A leading ~ is the home directory. A relative path is
# taken from the directory this file is in.
db = "~/Documents/inventory.db"
```

| Setting | Default | Overridden by |
|---------|---------|---------------|
| `db` | `~/.local/share/invy/invy.db` | `--db` |

An unknown setting or malformed TOML is an error that names the file. invy
reads the file before every command, including those given `--db`.

---

## Commands

### `invy add <name>`

Add a new item to the inventory.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Name of the item |

#### Flags
| Flag | Short | Description |
|------|-------|-------------|
| `--desc <text>` | `-d` | Item description |
| `--in <place>` | `-i` | Place to put the item in |
| `--kind <kind>` | `-k` | What sort of thing this is. Default: `thing` |
| `--count <n>` | | How many to add, each its own item. Default: 1. At least 1 |

#### Behavior
1. If `--in` is specified and place doesn't exist, **auto-create it**
2. A name already in the place adds another item, a duplicate if the
   description and kind match
3. The name and description are trimmed. A name cannot be empty or contain
   `/`. A blank description is no description
4. `--in /` and `--in root` add at root. An `--in @id` must exist
5. A refused add changes nothing, so an auto-created place is not left behind
6. `--count 3` adds three duplicates in one go

#### Output (human)
```
Added: hammer
  -> garage -> toolbox
```

With `--count 3`:
```
Added 3: hdmi cable
  -> cable storage
```

#### Output (JSON)

One object for one item. With `--count` above 1, an array of them.
```json
{
  "id": 5,
  "name": "hammer",
  "description": "claw hammer",
  "path": ["garage", "toolbox", "hammer"],
  "kind": "thing"
}
```

#### Output (CSV)

One row per item added.
```
id,name,description,kind,place
5,hammer,claw hammer,thing,toolbox
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Empty name, or a name containing `/` |
| 1 | `--in` is ambiguous |

#### Examples
```bash
# Add item at root
invy add "garage"

# Add with description
invy add "hammer" --desc "claw hammer"

# Add into place (auto-creates if needed)
invy add "screwdriver" --in toolbox

# Add with full path
invy add "wrench" --in "garage/toolbox"

# Add three identical cables
invy add "hdmi cable" --in "cable storage" --count 3

# Add a place and say what sort it is
invy add "bedroom" --kind room
invy add "dresser" --in bedroom --kind furniture
```

---

### `invy find [query]`

Search for items by name or description.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `query` | No | Search in fzf's syntax, described below. Required unless `--kind` is given |

#### Flags
| Flag | Short | Description |
|------|-------|-------------|
| `--kind <kind>` | `-k` | Only show items of this kind |

#### Behavior
1. Searches both `name` and `description` fields, never the path
2. The query uses fzf's syntax. Words separated by spaces must all match, in
   any order, and each one matches fuzzily, so `rpi psu` finds
   `raspberry pi power supply`
3. A word can be marked: `'word` matches exactly, `^word` at the start,
   `word$` at the end, and `!word` leaves out what it matches
4. Case is ignored
5. Results come best match first. Among equal scores, a match on the name
   alone beats one that needs the description, then results go by path
6. `--kind` alone returns every item of that kind, by path
7. A query and `--kind` together must both match
8. Neither a query nor `--kind` is an error

#### Output (human)

Each result is printed as the full slash-path on the first line, with the
description (if any) on an indented second line. The path is directly
pasteable into `invy show`. Duplicates print once, with a count.

```
garage/toolbox/hammer
  claw hammer

workshop/hammer
  ball peen

cable storage/hdmi cable ×3
```

#### Output (JSON)
```json
[
  {
    "id": 5,
    "name": "hammer",
    "description": "claw hammer",
    "path": ["garage", "toolbox", "hammer"],
    "child_count": 0,
    "kind": "thing"
  }
]
```

#### Output (CSV)
```
id,name,description,kind,path
5,hammer,claw hammer,thing,garage/toolbox/hammer
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success (including no results) |
| 1 | Neither a query nor `--kind` was given |

#### Examples
```bash
# Find by name
invy find hammer

# Find by description content
invy find "phillips"

# Words in any order, abbreviated
invy find "rpi psu"

# Cables, but not USB ones, and exactly "usb-c"
invy find "cable !usb"
invy find "'usb-c"

# Pipe to grep
invy find screw --json | jq '.[] | select(.path[0] == "garage")'

# Every room
invy find --kind room

# Rooms matching "bed"
invy find bed --kind room
```

---

### `invy list [place]`

List items, optionally within a specific place.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `place` | No | Place to list (default: root) |

#### Flags
| Flag | Short | Description |
|------|-------|-------------|
| `--recursive` | `-r` | List all descendants |

#### Behavior
1. Without argument: lists all root-level items
2. With place: lists direct children only (unless `--recursive`)
3. Shows item name, description, and child count if place
4. Duplicates share one row, with a count after the name. A place's child
   count still counts every item
5. At root, human output lists the places first, then the unsorted things
   (things at root) under an `Unsorted (N)` heading. `--recursive` does the
   same, with the unsorted things as branches of the heading. JSON and CSV
   keep one flat list
6. Items come in name order, ignoring case

#### Output (human)
```
NAME          KIND       DESCRIPTION      ITEMS
toolbox       box        red metal box    3
workbench     furniture  -                0

Unsorted (4)
NAME          KIND       DESCRIPTION      ITEMS
hammer        thing      claw hammer      -
hdmi cable ×3 thing      -                -
```

With `--recursive`, each place carries its kind glyph:
```
⌂ home [2]
├── ⌂ bedroom [1]
│   └── ▤ dresser [1]
│       └── ▣ sock drawer [1]
│           └── socks
└── ⌂ garage [4]
    ├── hdmi cable ×3
    └── ▣ toolbox [1]
        └── hammer (16oz claw)
Unsorted (2)
├── printer ink
└── tape
```

#### Output (JSON)
```json
[
  {
    "id": 2,
    "name": "toolbox",
    "description": "red metal box",
    "child_count": 3,
    "kind": "box"
  }
]
```

#### Output (CSV)
```
id,name,description,kind,child_count
2,toolbox,red metal box,box,3
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Place not found |

#### Examples
```bash
# List root items
invy list

# List items in a place
invy list toolbox

# List all items recursively
invy list --recursive

# List as JSON for scripting
invy list garage --json
```

---

### `invy show <item>`

Show detailed information about a specific item.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `item` | Yes | Item name or path |

#### Behavior
1. Shows item details including full path
2. If item is a place, shows child count
3. Resolves the reference as described in [References](#references)
4. If the item has duplicates, human output says how many share the place,
   the item included
5. If no exact name or path matches, searches names and descriptions as
   `find` does and prints `Did you mean:` followed by up to 10 candidate
   paths, best first, to stderr before exiting with code 1

#### Output (human)
```
Name:        hammer
Description: claw hammer
Place:       toolbox → garage
Kind:        thing
Created:     2024-01-15 10:30:00
Updated:     2024-01-15 10:30:00
```

One of three duplicates:
```
Name:        hdmi cable
Description: -
Place:       cable storage
Duplicates:  3 here
Kind:        thing
Created:     2024-01-15 10:30:00
Updated:     2024-01-15 10:30:00
```

For places:
```
Name:        toolbox
Description: red metal box
Place:       garage
Contains:    3 items
Kind:        box
Created:     2024-01-15 10:30:00
Updated:     2024-01-15 10:30:00
```

#### Output (JSON)
```json
{
  "id": 5,
  "name": "hammer",
  "description": "claw hammer",
  "path": ["garage", "toolbox", "hammer"],
  "child_count": 0,
  "kind": "thing",
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Item not found |
| 1 | Ambiguous reference |

#### Examples
```bash
# Show item
invy show hammer

# Show with path (if ambiguous)
invy show toolbox/hammer

# Show as JSON
invy show hammer --json
```

When the reference doesn't match exactly, suggestions are printed to stderr:

```
$ invy show "kvm switch"
error: item 'kvm switch' not found
Did you mean:
  garage/closet/8k displayport kvm switch
  office/desk/usb kvm switch
```

---

### `invy mv <item> <destination>`

Move an item to a different place.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `item` | Yes | Item to move |
| `destination` | Yes | Target place (use `/` for root) |

#### Flags
| Flag | Short | Description |
|------|-------|-------------|
| `--all` | | Move every duplicate `item` matches, not just one |

#### Behavior
1. Moves item to new place
2. If destination doesn't exist, **auto-create it**
3. Cannot move a place into itself or its descendants
4. Use `/`, `root` or an empty string as destination to move to root level
5. A refused move changes nothing, so an auto-created place is not left behind
6. When `item` matches several duplicates, one moves. `--all` moves them all.
   `--all` never makes an ambiguous reference acceptable

#### Output (human)
```
Moved: hammer
  garage -> toolbox -> workshop
```

One of several duplicates, then all of them:
```
Moved 1 of 3: hdmi cable
  cable storage -> desk
Moved 3: hdmi cable
  cable storage -> desk
```

#### Output (JSON)

One object for one item moved, as `show` prints it. An array when several moved.

#### Output (CSV)
```
id,name,description,kind,path
5,hammer,claw hammer,thing,workshop/hammer
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Item not found |
| 1 | Circular reference (moving into self/descendant) |

#### Examples
```bash
# Move to different place
invy mv hammer workshop

# Move to root level
invy mv hammer /

# Move with full paths
invy mv garage/toolbox/hammer workshop/bench

# Move every duplicate hdmi cable
invy mv "hdmi cable" "cable storage" --all
```

---

### `invy rm <item>`

Remove an item from the inventory.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `item` | Yes | Item to remove |

#### Flags
| Flag | Short | Description |
|------|-------|-------------|
| `--all` | | Remove every duplicate `item` matches, not just one |

#### Behavior
1. Removes the specified item
2. If item is a place with children, they move to root. Things there become
   unsorted. Rooms, furniture and boxes stay places at root
3. They keep their names and descriptions
4. When `item` matches several duplicates, one is removed. `--all` removes
   them all

#### Output (human)
```
Removed: garage
Now unsorted:
  - bike
  - hammer
Now at root:
  - toolbox
```

One of several duplicates:
```
Removed 1 of 3: hdmi cable
```

#### Output (JSON)
```json
{"removed": "toolbox", "count": 1, "orphaned": ["hammer", "screwdriver", "wrench"]}
```

#### Output (CSV)
```
removed,orphaned,count
toolbox,hammer;screwdriver;wrench,1
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Item not found |

#### Examples
```bash
# Remove item
invy rm hammer

# Remove place (its contents move to root)
invy rm toolbox
```

---

### `invy edit <item>`

Edit an existing item's name or description.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `item` | Yes | Item to edit |

#### Flags
| Flag | Short | Description |
|------|-------|-------------|
| `--name <text>` | `-n` | New name |
| `--desc <text>` | `-d` | New description |
| `--kind <kind>` | `-k` | New kind |

#### Behavior
1. At least one of `--name`, `--desc` or `--kind` must be provided
2. New name follows the name rules of `add`. It may match another item in the place
3. Use `--desc ""` to clear description. A blank description also clears it

#### Output (human)
```
Updated: hammer → ball-peen hammer
  description: "claw hammer" → "ball peen, 16oz"
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Item not found |
| 1 | Empty name, or a name containing `/` |
| 1 | No changes specified |
| 1 | Unknown kind |

#### Examples
```bash
# Change name
invy edit hammer --name "claw hammer"

# Change description
invy edit hammer --desc "16oz, fiberglass handle"

# Change both
invy edit hammer --name "ball-peen" --desc "ball peen hammer"

# Clear description
invy edit hammer --desc ""

# Reclassify a place
invy edit garage --kind room
```

---

### `invy tui`

Browse and edit the inventory in an interactive terminal UI. Alias: `invy ui`.
Plain `invy`, with no command, opens it too.

#### Arguments
None. `--db` applies. `--json` and `--csv` are ignored.

#### Layout
| Pane | Content |
|------|---------|
| Left | The place tree, one row per item |
| Right | Name, path, child count, timestamps and description of the selection. For a place, when anything inside it last changed |
| Bottom | The last result message, or the key hints |

A place row carries a `▸` or `▾` marker and its child count. A leaf row
carries a `·` marker. Duplicates share one row with a count, as in
`hdmi cable ×3`, in the tree and in search results. The details pane says how
many duplicates share the place.

**Unsorted** things, things at root, sit under an `Unsorted (N)` row at the
top of the tree. It folds like a place, but it is not an item: keys that change
an item do nothing on it. `p` into it moves the cut items to root, and `i` on it
adds at root. Rooms, furniture and boxes at root stay at the top level, so
giving an unsorted thing a place kind with `t` takes it out of Unsorted.

#### Keys
| Key | Action |
|-----|--------|
| `j` `k` `↓` `↑` | Move up and down |
| `g` `G` `Home` `End` | First and last row |
| `Ctrl-d` `Ctrl-u` | Half page down and up |
| `PageDown` `PageUp` | Full page down and up |
| `⏎` | Expand or collapse the selection |
| `l` `→` | Expand, or step into the first child |
| `h` `←` | Collapse, or select the place |
| `E` `C` | Expand all, collapse all |
| `Space` | Mark or unmark the row, every duplicate on it. The cursor stays |
| `v` | Visual mode: every row between where it started and the cursor is marked |
| `/` | Search by name and description, as `find` does |
| `a` | Add an item beside the selection, in the same place |
| `i` | Add an item inside the selection |
| `A` | Add an item at root |
| `+` | Add a duplicate of the selection |
| `-` | Remove one duplicate from the selection. Asks first when it is the last |
| `r` | Rename the selection |
| `e` | Edit the description. An empty value clears it |
| `x` | Cut the targets, to paste somewhere else |
| `X` | Cancel the cut |
| `p` | Paste the cut items into the selection |
| `m` | Move the targets to a place you type |
| `d` `Del` | Remove the targets, after a confirmation |
| `t` `T` | Next and previous kind. `k` is already "move up" |
| `R` | Reload from the database |
| `?` | Show the key list. Any key closes it |
| `q` `Ctrl-c` | Quit |
| `Esc` | Leave visual mode, else clear the marks, else clear the search. Never quits |

The **targets** of `x`, `m` and `d` are the marked items, or, with nothing
marked, the one item on the cursor row. The keys follow yazi's, which follow
vim's.

#### Behavior
1. Every change writes through the same functions the CLI commands use, so the
   name, path and move rules of `add`, `edit`, `mv` and `rm` all apply
2. Search shows a flat list of matches, best first, with the place path of
   each one
3. Tree keys do nothing while a search is active
4. A failed change leaves the database untouched and reports the reason in the
   bottom bar
5. Removing a place moves its contents to root, the same as `rm`
6. A key pressed on a duplicates row acts on one of them, the oldest, the same
   as the CLI without `--all`. Marking the row is how to act on all of them.
   A change that makes one differ, such as a new description, splits it onto
   its own row, and the cursor follows it
7. Marks survive a change of search. A marked or cut item never shares a row
   with one that is not, so cutting one of three duplicates splits it off
8. Visual mode lasts while the cursor moves. Any other key keeps the range
   marked, leaves visual mode, and then does its usual job, so `v` `G` `x`
   cuts every row from the cursor down
9. A cut stays until it is pasted or cancelled. A paste that is refused, such
   as a place into its own contents, moves nothing and keeps the cut
10. Marks clear after a cut, a move or a removal. The bottom bar shows how
    many items are marked and cut
11. The add prompt stays open after each name, adding to the same place, and
    its title counts what it has added. An empty line or `Esc` closes it, with
    the cursor on the last item added. A refused name stays in the prompt to
    fix. A name already in the place adds a duplicate, and the bar says how
    many there now are
12. `+` and `-` work on items that hold nothing, and ignore the marks
13. A place's "Changed" time is the latest update of anything inside it, at
    any depth. It is worked out each time, not stored. A move out of a place
    does not count

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | The user quit |
| 1 | The database could not be opened, or the terminal could not be set up |

#### Examples
```bash
# Browse the default database
invy

# The same, by name
invy tui

# Browse another database
invy tui --db ./garage.db
```

---

## Error Messages

All errors are written to stderr.

| Error | Message |
|-------|---------|
| Item not found | `Error: item 'NAME' not found` |
| Empty name | `Error: name cannot be empty` |
| Name with `/` | `Error: name cannot contain '/'` |
| Circular move | `Error: cannot move 'NAME' into itself or its descendants` |
| Ambiguous reference | `Error: 'REF' is ambiguous. Use a path or an @id:` then one `@ID  PATH  DESCRIPTION` line per match |
| No changes | `Error: no changes specified. Use --name, --desc or --kind` |
| Empty find | `Error: give a search term, a --kind, or both` |

---

## Database

SQLite database stored at `~/.local/share/invy/invy.db` (configurable with `db` in the config file, or `--db`).

### Schema
```sql
CREATE TABLE items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    place_id INTEGER REFERENCES items(id) ON DELETE SET NULL,
    kind TEXT NOT NULL DEFAULT 'thing',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_items_name ON items(name);
CREATE INDEX idx_items_kind ON items(kind);
CREATE INDEX idx_items_place ON items(place_id);
```

Note: `ON DELETE SET NULL` implements orphaning behavior for `rm` command.

### Migrations

`PRAGMA user_version` records how far a database file has come. `invy` migrates
on open, so any older file upgrades in place the first time a new build reads it.

| Version | Change |
|---------|--------|
| 1 | The original schema. The parent column was named `container_id` |
| 2 | `container_id` renamed to `place_id`. Indexes renamed to match |
| 3 | `kind` added, defaulting to `thing` for every existing row |
| 4 | The unique index on name and place dropped, so names can repeat in a place |
