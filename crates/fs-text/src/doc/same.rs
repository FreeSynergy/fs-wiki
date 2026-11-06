//! Id-blind document comparison.
//!
//! Pattern: Canonical form — both sides are reduced to the same normal form
//! (ids nil, container instances relabelled) and compared with `==`.

use super::attrs::BlockAttrs;
use super::container::Container;
use super::document::Document;
use crate::id::Uuid;
use crate::mark::Mark;
use std::collections::HashMap;

type Canon = Vec<(String, BlockAttrs, Vec<Mark>)>;

fn canon(d: &Document) -> Canon {
    let mut map: HashMap<(bool, u32), u32> = HashMap::default();
    let mut relabel = |list: bool, i: u32| {
        let n = u32::try_from(map.len()).unwrap_or(u32::MAX);
        *map.entry((list, i)).or_insert(n)
    };
    d.blocks
        .iter()
        .map(|b| {
            let mut a = b.attrs.clone();
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
                .cloned()
                .map(|mut m| (m.id = Uuid::nil(), m).1)
                .collect();
            (b.text.to_string(), a, marks)
        })
        .collect()
}

impl Document {
    /// Equality ignoring all ids (documents, blocks, marks); container instance numbers are
    /// compared by structure (relabelled in order of first appearance).
    #[must_use]
    pub fn same_content(&self, other: &Document) -> bool {
        canon(self) == canon(other)
    }
}
