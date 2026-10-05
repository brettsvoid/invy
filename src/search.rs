//! Search, shared by `find`, the suggestions `show` makes, and the TUI.
//!
//! A query uses fzf's syntax: words separated by spaces must all match, in any
//! order, each one fuzzily. `'word` matches exactly, `^word` at the start,
//! `word$` at the end, and `!word` leaves out what it matches. Names and
//! descriptions are searched, never the path, and case is ignored.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::model::Item;

/// The items that match `query`, best match first.
///
/// Among items that score the same, a match on the name alone beats one that
/// needs the description, and `tie_break` orders the rest. It is only called
/// for items that match. An empty query matches everything.
pub fn search<'a, K: Ord>(
    items: impl IntoIterator<Item = &'a Item>,
    query: &str,
    tie_break: impl Fn(&Item) -> K,
) -> Vec<&'a Item> {
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();

    let mut matches: Vec<(u32, bool, K, &Item)> = items
        .into_iter()
        .filter_map(|item| {
            let text = match &item.description {
                Some(desc) => format!("{} {desc}", item.name),
                None => item.name.clone(),
            };
            let score = pattern.score(Utf32Str::new(&text, &mut buf), &mut matcher)?;
            let in_name = pattern
                .score(Utf32Str::new(&item.name, &mut buf), &mut matcher)
                .is_some();
            Some((score, in_name, tie_break(item), item))
        })
        .collect();

    matches.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    matches.into_iter().map(|(_, _, _, item)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Kind;

    fn item(id: i64, name: &str, description: Option<&str>) -> Item {
        Item {
            id,
            name: name.to_string(),
            description: description.map(str::to_string),
            place_id: None,
            kind: Kind::Thing,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn names<'a>(found: &[&'a Item]) -> Vec<&'a str> {
        found.iter().map(|item| item.name.as_str()).collect()
    }

    #[test]
    fn descriptions_are_searched() {
        let items = [item(1, "hammer", Some("16oz claw")), item(2, "saw", None)];

        assert_eq!(names(&search(&items, "claw", |i| i.id)), ["hammer"]);
    }

    #[test]
    fn an_exact_word_must_appear_whole() {
        let items = [
            item(1, "usb-a to usb-c cable", None),
            item(2, "usb hub", Some("powered, 4 ports, c")),
        ];

        assert_eq!(
            names(&search(&items, "'usb-c", |i| i.id)),
            ["usb-a to usb-c cable"]
        );
    }

    #[test]
    fn a_match_in_the_name_beats_one_in_the_description() {
        let items = [
            item(1, "pi power supply", Some("5V, usb-c")),
            item(2, "usb-c cable", None),
        ];

        assert_eq!(
            names(&search(&items, "'usb-c", |i| i.id)),
            ["usb-c cable", "pi power supply"]
        );
    }

    #[test]
    fn ties_fall_back_to_the_tie_break() {
        let items = [item(2, "hammer", None), item(1, "hammer", None)];

        let found = search(&items, "hammer", |i| i.id);

        assert_eq!(found.iter().map(|i| i.id).collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn an_empty_query_matches_everything() {
        let items = [item(1, "hammer", None), item(2, "saw", None)];

        assert_eq!(search(&items, "", |i| i.id).len(), 2);
    }
}
