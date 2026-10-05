# Items have no quantity

An item is one physical thing, so three identical HDMI cables are three items, not one item with a count of 3. Items in the same place with the same name, description and kind that hold nothing are shown together as `×3`, but only on display. We chose this over a quantity column because moving, removing or describing one of several would otherwise mean splitting a row, and "identical" things often turn out to differ (one cable is 2m). Names are no longer unique within a place. An `@id` reference tells apart items that share a path.

## Considered Options

- **A quantity column.** Rejected because acting on some of a counted item needs a split, and an edit that makes two counted items match again needs a merge.
- **Unique names except between duplicates.** Rejected because describing one duplicate would give it the same name as a different item, and the edit would be refused.
