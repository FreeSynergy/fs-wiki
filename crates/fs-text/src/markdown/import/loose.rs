//! Which lists are loose?
//!
//! Pattern: Pre-pass analysis — a paragraph directly inside an item makes its list loose, and so
//! does a blank line right before the next item (items without paragraphs, like `*` or `* #`);
//! the result is a set of list event indices.

use pulldown_cmark::{Event, Tag, TagEnd};
use std::collections::HashSet;
use std::ops::Range;

/// Is the text between two items ended by a blank line (container marks `>` aside)?
fn blank_before_next(gap: &str) -> bool {
    let tail = gap.len()
        - gap
            .trim_end_matches(|c: char| c.is_whitespace() || c == '>')
            .len();
    gap[gap.len() - tail..].matches('\n').count() >= 2
}

pub(super) fn loose_lists(src: &str, events: &[(Event, Range<usize>)]) -> HashSet<usize> {
    let mut loose = HashSet::new();
    let mut stack: Vec<(bool, usize)> = Vec::new(); // (is_item, list_event_idx)
    let mut list_stack: Vec<(usize, Option<usize>)> = Vec::new(); // (event idx, last item start)
    for (i, (e, range)) in events.iter().enumerate() {
        match e {
            Event::Start(Tag::List(_)) => {
                list_stack.push((i, None));
                stack.push((false, i));
            }
            Event::Start(Tag::Item) => {
                let top = list_stack.last_mut();
                let list = top.as_ref().map_or(0, |t| t.0);
                if let Some((l, prev)) = top {
                    if prev.is_some_and(|p| blank_before_next(&src[p..range.start])) {
                        loose.insert(*l);
                    }
                    *prev = Some(range.start);
                }
                stack.push((true, list));
            }
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
