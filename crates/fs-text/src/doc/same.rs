//! Id-blind document comparison.
//!
//! Pattern: Canonical form — both sides are reduced to the same normal form (ids nil, container
//! instances relabelled, kept source dropped) and compared with `==`. The exporter uses the
//! same reduction without the layout style to prove that a file reads back as the model.

use super::attrs::BlockAttrs;
use super::block::Block;
use super::container::Container;
use super::document::Document;
use crate::id::Uuid;
use crate::mark::{Mark, MarkKind, MarkValue};
use std::collections::HashMap;

type Canon = Vec<(String, BlockAttrs, Vec<Mark>)>;

/// `extra` keys that only record layout, not content.
const STYLE_HINTS: [&str; 5] = ["glue", "dashes", "rule", "admonition", "callout-type"];

fn canon(blocks: &[Block], style: bool) -> Canon {
    let mut map: HashMap<(bool, u32), u32> = HashMap::default();
    let mut relabel = |list: bool, i: u32| {
        let n = u32::try_from(map.len()).unwrap_or(u32::MAX);
        *map.entry((list, i)).or_insert(n)
    };
    blocks
        .iter()
        .map(|b| {
            let mut a = b.attrs.clone();
            a.source = None;
            if !style {
                a.extra.retain(|k, _| !STYLE_HINTS.contains(&k.as_str()));
            }
            for c in &mut a.containers {
                let i = relabel(false, c.inst());
                c.set_inst(i);
                if let Container::Item { list, .. } = c {
                    *list = relabel(true, *list);
                }
            }
            let marks = b
                .marks
                .iter()
                .filter(|m| style || m.kind.is_markdown_native())
                .cloned()
                .map(|mut m| {
                    m.id = Uuid::nil();
                    if !style && matches!(m.kind, MarkKind::Emphasis | MarkKind::Strong) {
                        m.value = MarkValue::None;
                    }
                    m
                })
                .collect();
            (b.text.to_string(), a, marks)
        })
        .collect()
}

/// Do two block lists say the same in a file? Like [`Document::same_content`], but layout style
/// (hints, `_` vs `*`) and layer marks do not count.
pub(crate) fn same_in_file(a: &[Block], b: &[Block]) -> bool {
    canon(a, false) == canon(b, false)
}

/// Why block `i` of `a` does not read as block `i` of `b` (equal length): `None` if it does.
pub(crate) fn why_differs(a: &[Block], b: &[Block]) -> Vec<Option<&'static str>> {
    let (ca, cb) = (canon(a, false), canon(b, false));
    ca.iter()
        .zip(&cb)
        .map(|(x, y)| {
            if x == y {
                None
            } else if x.0 != y.0 {
                Some("its text reads back differently")
            } else if x.1.kind != y.1.kind {
                Some("it reads back as another kind of block")
            } else if x.1.containers != y.1.containers {
                Some("its nesting (list, quote) reads back differently")
            } else if x.2 != y.2 {
                Some("its marks (emphasis, links, code) read back differently")
            } else {
                Some("its attributes read back differently")
            }
        })
        .collect()
}

impl Document {
    /// Equality ignoring all ids (documents, blocks, marks) and the kept source texts; container
    /// instance numbers are compared by structure (relabelled in order of first appearance).
    #[must_use]
    pub fn same_content(&self, other: &Document) -> bool {
        canon(&self.blocks, true) == canon(&other.blocks, true)
    }
}
