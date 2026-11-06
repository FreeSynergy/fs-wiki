//! Link table (ADR 0042 §8): rows extracted from link marks; rename rewrites marks and the
//! readable path in the text.

use crate::doc::{Block, Document};
use crate::id::Uuid;
use crate::mark::*;
use crate::ops::Op;
use serde::{Deserialize, Serialize};

/// One row of the link table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkRow {
    pub source_doc: Uuid,
    pub source_block: Uuid,
    pub mark: Uuid,
    /// Char range of the link text in the source block.
    pub start: usize,
    pub end: usize,
    /// Readable path / URL.
    pub path: String,
    /// Resolved target (document / block), if known.
    pub target: Option<Uuid>,
    /// Link text as shown.
    pub title: String,
    pub internal: bool,
}

pub fn extract_links(doc: &Document) -> Vec<LinkRow> {
    let mut rows = Vec::new();
    for b in &doc.blocks {
        for m in b.marks.iter() {
            if !matches!(m.kind, MarkKind::Link | MarkKind::Image) {
                continue;
            }
            let Some(l) = m.link() else { continue };
            rows.push(LinkRow {
                source_doc: doc.id,
                source_block: b.id,
                mark: m.id,
                start: m.start,
                end: m.end,
                path: l.path.clone(),
                target: l.target,
                title: b.text.slice(m.start..m.end),
                internal: l.is_internal(),
            });
        }
    }
    rows
}

/// Rows pointing at `target` (back references come from the same table).
pub fn backlinks(rows: &[LinkRow], target: Uuid) -> Vec<&LinkRow> {
    rows.iter().filter(|r| r.target == Some(target)).collect()
}

/// `path` with the moved prefix replaced, or `None` if the link is not affected.
/// Matches the page itself, `old/…` (folder move) and keeps any `#fragment`.
pub fn moved_path(path: &str, old: &str, new: &str) -> Option<String> {
    let (page, frag) = match path.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (path, None),
    };
    let moved = if page == old {
        new.to_string()
    } else if let Some(rest) = page.strip_prefix(old).and_then(|r| r.strip_prefix('/')) {
        format!("{new}/{rest}")
    } else {
        return None;
    };
    Some(match frag {
        Some(f) => format!("{moved}#{f}"),
        None => moved,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct DocOps {
    pub doc: Uuid,
    /// Applied forward ops.
    pub ops: Vec<Op>,
    /// Inverse ops in undo order.
    pub inverse: Vec<Op>,
}

/// Rewrite all links `old -> new` in `docs` (marks and, where the text *is* the readable
/// path, the text too). Returns the applied ops per changed document, for the undo log.
pub fn rewrite_links(docs: &mut [Document], old: &str, new: &str) -> Vec<DocOps> {
    let mut result = Vec::new();
    for doc in docs.iter_mut() {
        let mut ops = Vec::new();
        let mut inverse = Vec::new();
        let block_ids: Vec<Uuid> = doc.blocks.iter().map(|b| b.id).collect();
        for bid in block_ids {
            // collect affected marks, process from the end so earlier ranges stay valid
            let mut todo: Vec<(Uuid, usize)> = doc
                .block(bid)
                .unwrap()
                .marks
                .iter()
                .filter(|m| matches!(m.kind, MarkKind::Link | MarkKind::Image) && m.link().and_then(|l| moved_path(&l.path, old, new)).is_some())
                .map(|m| (m.id, m.start))
                .collect();
            todo.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
            for (mid, _) in todo {
                let b: &Block = doc.block(bid).unwrap();
                let m = b.marks.get(mid).unwrap().clone();
                let l = m.link().unwrap();
                let new_path = moved_path(&l.path, old, new).unwrap();
                let shown = b.text.slice(m.start..m.end);
                let mut tmp = b.clone();
                if shown == l.path {
                    tmp.splice(m.start, m.end, &new_path);
                }
                let mut marks = tmp.marks.as_slice().to_vec();
                if let Some(mm) = marks.iter_mut().find(|x| x.id == mid) {
                    if let MarkValue::Link(lv) = &mut mm.value {
                        lv.path = new_path.clone();
                    }
                }
                let op = if shown == l.path {
                    Op::Edit { block: bid, start: m.start, end: m.end, insert: new_path.clone(), restore: Some(marks) }
                } else {
                    Op::SetMarks { block: bid, marks }
                };
                let inv = doc.apply(&op).expect("valid op");
                ops.push(op);
                inverse.push(inv);
            }
        }
        if !ops.is_empty() {
            inverse.reverse();
            result.push(DocOps { doc: doc.id, ops, inverse });
        }
    }
    result
}
