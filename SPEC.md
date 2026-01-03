# invy - Specification

A command-line tool for tracking home inventory with hierarchical containers.

## Core Concepts

### Items
Everything in invy is an **item**. An item has:
- **name** (required): unique identifier within its container
- **description** (optional): free-form text
- **container** (optional): parent item that holds this item

### Containers
A container is just an item that contains other items. There's no distinction between "item" and "container" - any item can hold other items.

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

### Paths
Items can be referenced by name. If ambiguous, use the full path with `/`:
```
toolbox/hammer
garage/toolbox/hammer
```

---

## Global Flags

All commands support these flags:

| Flag | Short | Description |
|------|-------|-------------|
| `--json` | `-j` | Output as JSON |
| `--csv` | | Output as CSV |
| `--db <path>` | | Use custom database file |

**Default database location:** `~/.invy.db`

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
| `--in <container>` | `-i` | Container to place item in |

#### Behavior
1. If `--in` is specified and container doesn't exist, **auto-create it**
2. Names must be unique within the same container
3. Names at root level must be unique among root items

#### Output (human)
```
Added: hammer
  └─ toolbox → garage
```

#### Output (JSON)
```json
{
  "id": 5,
  "name": "hammer",
  "description": "claw hammer",
  "path": ["garage", "toolbox", "hammer"]
}
```

#### Output (CSV)
```
id,name,description,container
5,hammer,claw hammer,toolbox
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Duplicate name in container |

#### Examples
```bash
# Add item at root
invy add "garage"

# Add with description
invy add "hammer" --desc "claw hammer"

# Add into container (auto-creates if needed)
invy add "screwdriver" --in toolbox

# Add with full path
invy add "wrench" --in "garage/toolbox"
```

---

### `invy find <query>`

Search for items by name or description.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `query` | Yes | Search term (substring match) |

#### Behavior
1. Searches both `name` and `description` fields
2. Case-insensitive substring matching
3. Returns all matches with their full paths

#### Output (human)
```
hammer (claw hammer)
  └─ toolbox → garage

hammer (ball peen)
  └─ workshop
```

#### Output (JSON)
```json
[
  {
    "id": 5,
    "name": "hammer",
    "description": "claw hammer",
    "path": ["garage", "toolbox", "hammer"]
  }
]
```

#### Output (CSV)
```
id,name,description,path
5,hammer,claw hammer,garage/toolbox/hammer
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success (including no results) |

#### Examples
```bash
# Find by name
invy find hammer

# Find by description content
invy find "phillips"

# Pipe to grep
invy find screw --json | jq '.[] | select(.path[0] == "garage")'
```

---

### `invy list [container]`

List items, optionally within a specific container.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `container` | No | Container to list (default: root) |

#### Flags
| Flag | Short | Description |
|------|-------|-------------|
| `--recursive` | `-r` | List all descendants |

#### Behavior
1. Without argument: lists all root-level items
2. With container: lists direct children only (unless `--recursive`)
3. Shows item name, description, and child count if container

#### Output (human)
```
NAME          DESCRIPTION      ITEMS
toolbox       red metal box    3
workbench     -                0
hammer        claw hammer      -
```

#### Output (JSON)
```json
[
  {
    "id": 2,
    "name": "toolbox",
    "description": "red metal box",
    "child_count": 3
  }
]
```

#### Output (CSV)
```
id,name,description,child_count
2,toolbox,red metal box,3
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Container not found |

#### Examples
```bash
# List root items
invy list

# List items in container
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
2. If item is a container, shows child count
3. Resolves ambiguous names (errors if multiple matches)

#### Output (human)
```
Name:        hammer
Description: claw hammer
Location:    toolbox → garage
Created:     2024-01-15 10:30:00
Updated:     2024-01-15 10:30:00
```

For containers:
```
Name:        toolbox
Description: red metal box
Location:    garage
Contains:    3 items
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
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Item not found |
| 1 | Ambiguous name (multiple matches) |

#### Examples
```bash
# Show item
invy show hammer

# Show with path (if ambiguous)
invy show toolbox/hammer

# Show as JSON
invy show hammer --json
```

---

### `invy mv <item> <destination>`

Move an item to a different container.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `item` | Yes | Item to move |
| `destination` | Yes | Target container (use `/` for root) |

#### Behavior
1. Moves item to new container
2. If destination doesn't exist, **auto-create it**
3. Cannot move a container into itself or its descendants
4. Use `/` or `root` as destination to move to root level

#### Output (human)
```
Moved: hammer
  toolbox → garage
  to: workshop
```

#### Exit Codes
| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Item not found |
| 1 | Circular reference (moving into self/descendant) |
| 1 | Name conflict in destination |

#### Examples
```bash
# Move to different container
invy mv hammer workshop

# Move to root level
invy mv hammer /

# Move with full paths
invy mv garage/toolbox/hammer workshop/bench
```

---

### `invy rm <item>`

Remove an item from the inventory.

#### Arguments
| Argument | Required | Description |
|----------|----------|-------------|
| `item` | Yes | Item to remove |

#### Behavior
1. Removes the specified item
2. If item is a container with children: **orphan children to root level**
3. Orphaned items retain their names and descriptions

#### Output (human)
```
Removed: toolbox
Orphaned 3 items to root:
  - hammer
  - screwdriver
  - wrench
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

# Remove container (orphans contents)
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

#### Behavior
1. At least one of `--name` or `--desc` must be provided
2. New name must be unique within container
3. Use `--desc ""` to clear description

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
| 1 | Name conflict |
| 1 | No changes specified |

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
```

---

## Error Messages

All errors are written to stderr.

| Error | Message |
|-------|---------|
| Item not found | `Error: item 'NAME' not found` |
| Duplicate name | `Error: item 'NAME' already exists in CONTAINER` |
| Circular move | `Error: cannot move 'NAME' into itself or its descendants` |
| Ambiguous name | `Error: 'NAME' is ambiguous. Use full path: PATH1, PATH2` |
| No changes | `Error: no changes specified. Use --name or --desc` |

---

## Database

SQLite database stored at `~/.invy.db` (configurable with `--db`).

### Schema
```sql
CREATE TABLE items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    container_id INTEGER REFERENCES items(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_items_name ON items(name);
CREATE INDEX idx_items_container ON items(container_id);
CREATE UNIQUE INDEX idx_items_name_container ON items(name, container_id);
```

Note: `ON DELETE SET NULL` implements orphaning behavior for `rm` command.
