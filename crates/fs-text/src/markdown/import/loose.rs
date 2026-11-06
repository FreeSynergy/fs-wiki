//! Which lists are loose?
//!
//! Pattern: Pre-pass analysis — a paragraph directly inside an item makes its list loose;
//! the result is a set of list event indices.

use pulldown_cmark::{Event, Tag, TagEnd};
use std::collections::HashSet;
use std::ops::Range;

pub(super) fn loose_lists(events: &[(Event, Range<usize>)]) -> HashSet<usize> {
    let mut loose = HashSet::new();
    let mut stack: Vec<(bool, usize)> = Vec::new(); // (is_item, list_event_idx)
    let mut list_stack: Vec<usize> = Vec::new();
    for (i, (e, _)) in events.iter().enumerate() {
        match e {
            Event::Start(Tag::List(_)) => {
                list_stack.push(i);
                stack.push((false, i));
            }
            Event::Start(Tag::Item) => stack.push((true, *list_stack.last().unwrap_or(&0))),
            Event::Start(Tag::Paragraph) => {
                if let Some((true, l)) = stack.last() {
                    loose.insert(*l);
                }
                stack.push((false, 0));
            }
            Event::Start(_) => stack.push((false, 0)),
            Event::End(t) => {
                if matches!(t, TagEnd::List(_)) {
                    list_stack.pop();
                }
                stack.pop();
            }
            _ => {}
        }
    }
    loose
}
