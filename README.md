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
