# invy

A home inventory that helps a household remember where things are. Things sit inside other things, so the inventory is a tree of places.

## Language

**Item**:
One physical thing. Two identical cables are two items, never one item with a count.
_Avoid_: Entry, record, object

**Duplicates**:
Items that are interchangeable: in the same place, with the same name, description and kind, and holding nothing. Giving one a different description stops it being a duplicate.
_Avoid_: Copies, stack, group

**Place**:
An item that holds other items. Any item can become a place by having something put in it.
_Avoid_: Container, location, parent

**Kind**:
What sort of item something is: a room, furniture, a box, or a thing. It describes, and never restricts what goes where.
_Avoid_: Type, category

**Path**:
The names from the top of the tree down to an item, joined with `/`, such as `garage/toolbox/hammer`. Several items can share a path, so a path names a spot in the tree, not always one item.

**Id**:
A number that names one item for its whole life, written `@14`. It is the only reference that can tell apart two items with the same path.

**Root**:
The top of the tree. An item at root is in no place. Rooms, furniture and boxes may belong there; things do not.

**Unsorted**:
A thing at root, meaning its place has not been recorded yet. Unsorted items are a to-do list, not a place.
_Avoid_: Inbox, loose, orphaned
