# invy

A CLI tool for tracking home inventory with hierarchical places.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Add items
invy add garage --kind room
invy add toolbox --in garage --kind box
invy add hammer --in garage/toolbox --desc "claw hammer"

# List items
invy list                  # list root items
invy list garage           # list items in garage
invy list --recursive      # show full tree

# Search
invy find hammer
invy find --kind room       # every room
invy find bed --kind room   # rooms matching "bed"

# Show details
invy show hammer

# Move items
invy mv hammer kitchen     # move to different place
invy mv hammer /           # move to root

# Edit items
invy edit hammer --name "claw hammer" --desc "16oz"
invy edit garage --kind room

# Remove items
invy rm hammer
```

Output formats: `--json`, `--csv`

## Kinds

Every item has a kind. The first three describe a place, and `thing` is the
default for everything you put in one.

| Kind | Unicode | Nerd | Covers |
| ---- | ------- | ---- | ------ |
| `room` | `⌂` | `󰋞` | A room, a loft, a shed, a garden |
| `furniture` | `▤` | `󰽊` | A cupboard, a dresser, a shelf, a workbench |
| `box` | `▣` | `󰏗` | A box, a bag, a case, a toolbox |
| `thing` | | | Anything you put in a place |

```
$ invy list --recursive
⌂ home [2]
├── ⌂ bedroom [1]
│   └── ▤ dresser [1]
│       └── ▣ sock drawer [1]
│           └── socks
└── ⌂ garage [1]
    └── ▣ toolbox [1]
        └── hammer (16oz claw)
```

## Glyphs

`--glyphs` picks what the tree is drawn with. `INVY_GLYPHS` sets it once, and
the flag wins over the environment.

| Set | Kinds | Folds | Branches | Needs |
| --- | ----- | ----- | -------- | ----- |
| `unicode` (default) | `⌂ ▤ ▣` | `▾ ▸ ·` | `├── └── │` | any font |
| `nerd` | `󰋞 󰽊 󰏗` | `  ` | `├── └── │` | a patched font |
| `ascii` | none | `v > -` | `\|--` `` `-- `` `\|` | nothing |

```bash
invy list --recursive --glyphs nerd
export INVY_GLYPHS=nerd            # or set it once
invy list --recursive --glyphs ascii | mail -s inventory me@example.com
```

With `nerd`, kinds are Material Design icons (`md-home_variant`, `md-dresser`,
`md-package_variant_closed`) and folds are Codicons, the set VS Code draws its
own tree views with. All of them sit in the Private Use Area, so a terminal
without a patched font shows blank boxes.

Use a **Mono** Nerd Font variant. The others draw icons double-width while the
terminal reserves one cell, which breaks the tree alignment.

## Interactive mode

```bash
invy tui
```

`invy tui` opens a terminal UI over the same database. The left pane is the
place tree, the right pane shows the selected item.

| Key               | Action                                     |
| ----------------- | ------------------------------------------ |
| `j` `k` `↓` `↑`   | Move up and down                           |
| `g` `G`           | First and last row                          |
| `Ctrl-d` `Ctrl-u` | Half page down and up                       |
| `⏎` `Space`       | Expand or collapse                          |
| `l` `h`           | Expand, or collapse and go to the place |
| `E` `C`           | Expand all, collapse all                    |
| `/`               | Search names and descriptions               |
| `a` `A`           | Add inside the selection, add at root       |
| `r` `d`           | Rename, edit the description                |
| `t` `T`           | Next and previous kind                      |
| `m`               | Move to another place                   |
| `x` `Del`         | Remove (asks first)                         |
| `R`               | Reload from the database                    |
| `?`               | Show the key list                           |
| `q` `Esc`         | Quit                                        |
