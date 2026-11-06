//! Flat blocks with containers to nested blocks.
//!
//! Pattern: Composite builder — `fs-text` keeps blocks flat, each with the list of containers it
//! sits in (outermost first). Runs of blocks that share the container at one depth become one
//! quote, callout, footnote or list; items of a list are told apart by their instance.

use fs_components_widgets::{Block as Out, Inline, ListItem, Mark, MarkKind};
use fs_text::{Block, Container};

use super::blocks::{leaf, paragraph};

/// How many containers deep blocks are nested before the rest is shown flat.
const MAX_DEPTH: usize = 24;

/// The nested blocks of `blocks`, looking at the container at `depth`.
pub(super) fn nest(blocks: &[&Block], depth: usize) -> Vec<Out> {
    let mut out = Vec::new();
    if depth >= MAX_DEPTH {
        // Deeper containers are not drawn as containers (a stack of thousands would overflow the
        // stack of the layout); their blocks are still shown.
        return blocks.iter().map(|b| leaf(b)).collect();
    }
    let mut i = 0;
    while i < blocks.len() {
        let Some(first) = blocks[i].attrs.containers.get(depth) else {
            out.push(leaf(blocks[i]));
            i += 1;
            continue;
        };
        let key = group_key(first);
        let mut j = i + 1;
        while j < blocks.len()
            && blocks[j]
                .attrs
                .containers
                .get(depth)
                .is_some_and(|c| group_key(c) == key)
        {
            j += 1;
        }
        out.push(group(first, &blocks[i..j], depth));
        i = j;
    }
    out
}

/// What blocks must share to be in the same group: the kind and the list or instance.
fn group_key(c: &Container) -> (u8, u32) {
    match c {
        Container::Quote { inst } => (0, *inst),
        Container::Callout { inst, .. } => (1, *inst),
        Container::Item { list, .. } => (2, *list),
        Container::Footnote { inst, .. } => (3, *inst),
    }
}

/// The element of one group of blocks that share the container `first` at `depth`.
fn group(first: &Container, blocks: &[&Block], depth: usize) -> Out {
    match first {
        Container::Quote { .. } => Out::Quote(nest(blocks, depth + 1)),
        Container::Callout { ty, title, .. } => {
            let heading = if title.is_empty() { ty } else { title };
            let mut inner = vec![Out::Paragraph(Inline::new(
                heading.clone(),
                vec![Mark::new(MarkKind::Strong, 0, heading.chars().count())],
            ))];
            inner.extend(nest(blocks, depth + 1));
            Out::Quote(inner)
        }
        Container::Footnote { label, .. } => {
            let mut inner = vec![paragraph(&format!("[^{label}]"))];
            inner.extend(nest(blocks, depth + 1));
            Out::Quote(inner)
        }
        Container::Item { number, .. } => list(number.is_some(), blocks, depth),
    }
}

/// A list: split the run into items at every change of the item's instance.
fn list(ordered: bool, blocks: &[&Block], depth: usize) -> Out {
    let inst_of = |b: &Block| match b.attrs.containers.get(depth) {
        Some(Container::Item { inst, .. }) => *inst,
        _ => 0,
    };
    let mut items = Vec::new();
    let mut start = 1;
    let mut i = 0;
    while i < blocks.len() {
        let inst = inst_of(blocks[i]);
        let mut j = i + 1;
        while j < blocks.len() && inst_of(blocks[j]) == inst {
            j += 1;
        }
        if let Some(Container::Item { number, task, .. }) = blocks[i].attrs.containers.get(depth) {
            if items.is_empty() {
                start = number.unwrap_or(1);
            }
            let item = ListItem::new(nest(&blocks[i..j], depth + 1));
            items.push(task.map_or(item.clone(), |done| item.task(done)));
        }
        i = j;
    }
    Out::List {
        ordered,
        start,
        items,
    }
}
