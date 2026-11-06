//! Block alignment for block documents.
//!
//! Pattern: Template method step — blocks are compared by their exported form (Myers), equal
//! blocks keep id and layers, changed ones get text edits plus formatting marks from the file,
//! new / removed ones become `InsertBlock` / `RemoveBlock`.

use super::diff::{segments, Seg};
use super::plan::block_text_ops;
use crate::doc::{Block, Document, Source};
use crate::format::Exporter;
use crate::id::Uuid;
use crate::mark::Mark;
use crate::ops::{Command, Op};
use std::collections::HashMap;

/// The new document with the block ids its kept sources point to replaced by the ids the blocks
/// have after the plan ran (a block that equals an old one keeps the old id).
fn with_final_ids(doc: &Document, new_doc: &Document, segs: &[Seg]) -> Document {
    let mut final_ids: Vec<Uuid> = new_doc.blocks.iter().map(|b| b.id).collect();
    for g in segs {
        for k in 0..g.ol.min(g.nl) {
            final_ids[g.ni + k] = doc.blocks[g.oi + k].id;
        }
    }
    let map: HashMap<Uuid, Uuid> = new_doc.blocks.iter().map(|b| b.id).zip(final_ids).collect();
    let mut nd = new_doc.clone();
    for b in &mut nd.blocks {
        let fp = b.fingerprint();
        if let Some(src) = b.attrs.source.as_mut() {
            src.prev = src.prev.map(|p| map[&p]);
            for r in &mut src.rest {
                r.0 = map[&r.0];
            }
            src.hash = Source::seal(&fp, &src.rest, &src.text, src.own);
        }
    }
    nd
}

pub(super) fn plan_blocks(doc: &Document, new_doc: &Document, exporter: &dyn Exporter) -> Vec<Op> {
    // blocks are compared by their canonical form: the kept source text (blank lines around a
    // block) is not what makes a block different
    let key = |b: &Block| {
        let mut plain = b.clone();
        plain.attrs.source = None;
        exporter.export_blocks(&[plain])
    };
    let ok: Vec<String> = doc.blocks.iter().map(key).collect();
    let nk: Vec<String> = new_doc.blocks.iter().map(key).collect();
    let segs = segments(&ok, &nk);
    let new_doc = &with_final_ids(doc, new_doc, &segs);
    let mut work = doc.clone();
    let mut ops: Vec<Op> = Vec::new();
    let mut pos = 0usize;
    let mut push = |work: &mut Document, op: Op| {
        op.apply(work).expect("planned op is valid");
        ops.push(op);
    };
    for g in segs {
        let (ol, ni, nl, equal) = (g.ol, g.ni, g.nl, g.equal);
        let paired = ol.min(nl);
        for k in 0..paired {
            let id = work.blocks[pos].id;
            let nb = &new_doc.blocks[ni + k];
            if !equal {
                for op in
                    block_text_ops(id, &work.blocks[pos].text.to_string(), &nb.text.to_string())
                {
                    push(&mut work, op);
                }
                // formatting comes from the file, layers stay
                let cur = &work.blocks[pos];
                let mut marks: Vec<Mark> = cur
                    .marks
                    .iter()
                    .filter(|m| !m.kind.is_markdown_native())
                    .cloned()
                    .collect();
                marks.extend(nb.marks.iter().cloned());
                let old_native: Vec<&Mark> = cur
                    .marks
                    .iter()
                    .filter(|m| m.kind.is_markdown_native())
                    .collect();
                let same = old_native.len() == nb.marks.len()
                    && old_native.iter().zip(nb.marks.iter()).all(|(a, b)| {
                        a.kind == b.kind
                            && a.start == b.start
                            && a.end == b.end
                            && a.value == b.value
                    });
                if !same {
                    push(&mut work, Op::SetMarks { block: id, marks });
                }
            }
            if work.blocks[pos].attrs != nb.attrs {
                push(
                    &mut work,
                    Op::SetAttrs {
                        block: id,
                        attrs: nb.attrs.clone(),
                    },
                );
            }
            pos += 1;
        }
        for _ in paired..ol {
            let id = work.blocks[pos].id;
            push(&mut work, Op::RemoveBlock { block: id });
        }
        for k in paired..nl {
            let nb = new_doc.blocks[ni + k].clone();
            push(
                &mut work,
                Op::InsertBlock {
                    index: pos,
                    block: nb,
                },
            );
            pos += 1;
        }
    }
    ops
}
