//! Applying a [`LinkRewriter`] to documents as undoable ops.
//!
//! Pattern: Command producer — marks and (where the text *is* the readable path) the text
//! are changed through [`Op`]s, so the caller gets forward and inverse ops per document.

use super::rewriter::{LinkRewriter, MovePrefix};
use super::title::rewrite_title;
use crate::doc::{Block, Container, Document};
use crate::id::Uuid;
use crate::mark::{MarkKind, MarkValue};
use crate::ops::Op;

/// Ops applied to one document by a rewrite.
#[derive(Clone, Debug, PartialEq)]
pub struct DocOps {
    /// Document id.
    pub doc: Uuid,
    /// Applied forward ops.
    pub ops: Vec<Op>,
    /// Inverse ops in undo order.
    pub inverse: Vec<Op>,
}

/// Rewrite all links `old -> new` in `docs`. Returns the applied ops per changed document.
pub fn rewrite_links(docs: &mut [Document], old: &str, new: &str) -> Vec<DocOps> {
    rewrite_links_with(docs, &MovePrefix::new(old, new))
}

/// Rewrite all links the `rewriter` changes (marks and, where the text *is* the readable
/// path, the text too). Returns the applied ops per changed document, for the undo log.
///
/// # Panics
/// Never for documents of this crate: the generated ops are valid by construction.
pub fn rewrite_links_with(docs: &mut [Document], rewriter: &dyn LinkRewriter) -> Vec<DocOps> {
    let mut result = Vec::new();
    for doc in docs.iter_mut() {
        let mut ops = Vec::new();
        let mut inverse = Vec::new();
        let block_ids: Vec<Uuid> = doc.blocks.iter().map(|b| b.id).collect();
        for bid in block_ids {
            // process from the end so earlier ranges stay valid
            let mut todo: Vec<(Uuid, usize)> = doc
                .block(bid)
                .expect("block exists")
                .marks
                .iter()
                .filter(|m| {
                    matches!(m.kind, MarkKind::Link | MarkKind::Image)
                        && m.link().and_then(|l| rewriter.rewrite(&l.path)).is_some()
                })
                .map(|m| (m.id, m.start))
                .collect();
            todo.sort_by_key(|(_, s)| std::cmp::Reverse(*s));
            for (mid, _) in todo {
                let op = rewrite_op(doc.block(bid).expect("block exists"), mid, rewriter);
                let inv = doc.apply(&op).expect("valid op");
                ops.push(op);
                inverse.push(inv);
            }
        }
        // links in callout titles are text of the container, not marks
        for bid in doc.blocks.iter().map(|b| b.id).collect::<Vec<_>>() {
            let mut attrs = doc.block(bid).expect("block exists").attrs.clone();
            let mut changed = false;
            for c in &mut attrs.containers {
                if let Container::Callout { title, .. } = c {
                    if let Some(new) = rewrite_title(title, rewriter) {
                        *title = new;
                        changed = true;
                    }
                }
            }
            if changed {
                let op = Op::SetAttrs { block: bid, attrs };
                let inv = doc.apply(&op).expect("valid op");
                ops.push(op);
                inverse.push(inv);
            }
        }
        if !ops.is_empty() {
            inverse.reverse();
            result.push(DocOps {
                doc: doc.id,
                ops,
                inverse,
            });
        }
    }
    result
}

/// The op that rewrites link mark `mid` of block `b`.
fn rewrite_op(b: &Block, mid: Uuid, rewriter: &dyn LinkRewriter) -> Op {
    let m = b.marks.get(mid).expect("mark exists").clone();
    let l = m.link().expect("link value");
    let new_path = rewriter.rewrite(&l.path).expect("affected");
    let shown = b.text.slice(m.start..m.end);
    let mut tmp = b.clone();
    if shown == l.path {
        tmp.splice(m.start, m.end, &new_path);
    }
    let mut marks = tmp.marks.as_slice().to_vec();
    if let Some(mm) = marks.iter_mut().find(|x| x.id == mid) {
        if let MarkValue::Link(lv) = &mut mm.value {
            lv.path.clone_from(&new_path);
        }
    }
    if shown == l.path {
        Op::Edit {
            block: b.id,
            start: m.start,
            end: m.end,
            insert: new_path,
            restore: Some(marks),
        }
    } else {
        Op::SetMarks { block: b.id, marks }
    }
}
