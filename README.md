# invy

A CLI tool for tracking home inventory with hierarchical containers.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Add items
invy add garage
invy add toolbox --in garage
invy add hammer --in garage/toolbox --desc "claw hammer"

# List items
invy list                  # list root items
invy list garage           # list items in garage
invy list --recursive      # show full tree

# Search
invy find hammer

# Show details
invy show hammer

# Move items
invy mv hammer kitchen     # move to different container
invy mv hammer /           # move to root

# Edit items
invy edit hammer --name "claw hammer" --desc "16oz"

# Remove items
invy rm hammer
```

Output formats: `--json`, `--csv`

## Interactive mode

```bash
invy tui
```

`invy tui` opens a terminal UI over the same database. The left pane is the
container tree, the right pane shows the selected item.

| Key               | Action                                     |
| ----------------- | ------------------------------------------ |
| `j` `k` `↓` `↑`   | Move up and down                           |
| `g` `G`           | First and last row                          |
| `Ctrl-d` `Ctrl-u` | Half page down and up                       |
| `⏎` `Space`       | Expand or collapse                          |
| `l` `h`           | Expand, or collapse and go to the container |
| `E` `C`           | Expand all, collapse all                    |
| `/`               | Search names and descriptions               |
| `a` `A`           | Add inside the selection, add at root       |
| `r` `d`           | Rename, edit the description                |
| `m`               | Move to another container                   |
| `x` `Del`         | Remove (asks first)                         |
| `R`               | Reload from the database                    |
| `?`               | Show the key list                           |
| `q` `Esc`         | Quit                                        |
