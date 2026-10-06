# invy

A CLI tool for remembering where you put things. Things sit inside places,
places inside other places: `garage/toolbox/hammer`.

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
invy add "hdmi cable" --in "cable storage" --count 3   # three of them

# List items
invy list                  # root places, then the unsorted things
invy list garage           # list items in garage
invy list --recursive      # show full tree

# Search, with fzf's syntax
invy find hammer
invy find "rpi psu"         # words in any order, abbreviated
invy find "cable !usb"      # cables, but not usb ones
invy find "'usb-c"          # exactly "usb-c"
invy find --kind room       # every room
invy find bed --kind room   # rooms matching "bed"

# Show details
invy show hammer
invy show @14              # by id, when two items share a path

# Move items
invy mv hammer kitchen     # move to different place
invy mv hammer /           # move to root
invy mv "hdmi cable" desk          # one of the three
invy mv "hdmi cable" desk --all    # all of them

# Edit items
invy edit hammer --name "claw hammer" --desc "16oz"
invy edit garage --kind room

# Remove items
invy rm hammer
```

Output formats: `--json`, `--csv`

## Duplicates

An item is one physical thing, so three identical cables are three items.
Items in the same place with the same name, description and kind, holding
nothing, are **duplicates**. invy shows them as one line, `hdmi cable ×3`, and
a command given their name acts on one of them unless you pass `--all`. JSON
and CSV list every item with its own id.

When two items share a path but are not duplicates, say which one you mean
with its id, `@14`. The error lists the ids to choose from.

## Unsorted

A `thing` at root is **unsorted**: invy does not know where it is yet. `list`
and the TUI show unsorted things apart, as a to-do list. Put each one in a
place, or give a top-level place its kind (`room`, `furniture` or `box`) so it
is not counted as unsorted.

## Naming things

Search finds what the words say, so a little consistency pays off:

- Name what it is, in plain words, with the general word last:
  `usb-a to usb-c cable`, `raspberry pi power supply`, `phillips screwdriver`.
- Spell common names one way: `usb-c`, `hdmi`, `mini displayport`. Then
  `invy find "'usb-c"` finds every one.
- Put what tells otherwise identical things apart in the description: length,
  wattage, colour, version. Things with the same description stay duplicates.
- Never number things. A third cable is `--count`, or `+` in the TUI, not
  `hdmi cable (3)`.

## Kinds

Every item has a kind. The first three describe a place, and `thing` is the
default for everything you put in one.

| Kind | Nerd | Unicode | Covers |
| ---- | ---- | ------- | ------ |
| `room` | `󰋞` | `⌂` | A room, a loft, a shed, a garden |
| `furniture` | `󰽊` | `▤` | A cupboard, a dresser, a shelf, a workbench |
| `box` | `󰏗` | `▣` | A box, a bag, a case, a toolbox |
| `thing` | | | Anything you put in a place |

```
$ invy list --recursive
󰋞 home [2]
├── 󰋞 bedroom [1]
│   └── 󰽊 dresser [1]
│       └── 󰏗 sock drawer [1]
│           └── socks
└── 󰋞 garage [1]
    └── 󰏗 toolbox [1]
        └── hammer (16oz claw)
```

## Glyphs

`--glyphs` picks what the tree is drawn with. `INVY_GLYPHS` sets it once, and
the flag wins over the environment.

| Set | Kinds | Folds | Branches | Count, mark | Needs |
| --- | ----- | ----- | -------- | ----------- | ----- |
| `nerd` (default) | `󰋞 󰽊 󰏗` | `  ` | `├── └── │` | `× ▍` | a patched font |
| `unicode` | `⌂ ▤ ▣` | `▾ ▸ ·` | `├── └── │` | `× ▍` | any font |
| `ascii` | none | `v > -` | `\|--` `` `-- `` `\|` | `x *` | nothing |

Without a patched font, switch to the portable set:

```bash
invy list --recursive --glyphs unicode
export INVY_GLYPHS=unicode         # or set it once
invy list --recursive --glyphs ascii | mail -s inventory me@example.com
```

With `nerd`, kinds are Material Design icons (`md-home_variant`, `md-dresser`,
`md-package_variant_closed`) and folds are Codicons, the set VS Code draws its
own tree views with. All of them sit in the Private Use Area, so a terminal
without a patched font shows blank boxes.

Use a **Mono** Nerd Font variant. The others draw icons double-width while the
terminal reserves one cell, which breaks the tree alignment.

## Configuration

invy reads `~/.config/invy/config.toml` (or `$XDG_CONFIG_HOME/invy/config.toml`)
if it exists. Create it yourself to keep the database somewhere other than
`~/.local/share/invy/invy.db`:

```toml
db = "~/Documents/inventory.db"
```

`--db` still beats it for a single command.

## Interactive mode

```bash
invy
```

Plain `invy`, or `invy tui`, opens a terminal UI over the same database. The
left pane is the place tree, with the unsorted things under an `Unsorted` row at
the top. The right pane shows the selected item. The keys follow
[yazi](https://yazi-rs.github.io)'s, which follow vim's.

| Key               | Action                                               |
| ----------------- | ---------------------------------------------------- |
| `j` `k` `↓` `↑`   | Move up and down                                     |
| `g` `G`           | First and last row                                   |
| `Ctrl-d` `Ctrl-u` | Half page down and up                                |
| `⏎`               | Expand or collapse                                   |
| `l` `h`           | Expand, or collapse and go to the place              |
| `E` `C`           | Expand all, collapse all                             |
| `/`               | Search names and descriptions, as `find` does        |
| `Space`           | Mark or unmark the row                               |
| `v`               | Visual mode: mark every row the cursor passes        |
| `x` `X`           | Cut the marked items, or cancel the cut              |
| `p`               | Paste the cut items into the selection               |
| `m`               | Move the marked items to a place you type            |
| `d` `Del`         | Remove the marked items (asks first)                 |
| `a` `i` `A`       | Add beside the selection, inside it, at root         |
| `+` `-`           | Add or remove a duplicate                            |
| `r` `e`           | Rename, edit the description                         |
| `t` `T`           | Next and previous kind                               |
| `R`               | Reload from the database                             |
| `?`               | Show the key list                                    |
| `Esc`             | Leave visual mode, clear the marks, clear the search |
| `q`               | Quit                                                 |

With nothing marked, `x`, `m` and `d` act on the item under the cursor. On a
`×3` row that is one of the three. Mark the row to act on all of them.

The add prompt stays open, so you can type a box's contents one name after
another. An empty line or `Esc` closes it.

Moving a box of cables into new storage looks like this: `/cable` `⏎` to find
them, `v` `G` `Esc` to mark them all, then `Space` on any to leave out, `x` to
cut, `Esc` to clear the search, and `p` on the storage.
