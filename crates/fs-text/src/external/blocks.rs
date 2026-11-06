//! Block alignment for block documents.
//!
//! Pattern: Template method step — blocks are compared by their exported form (Myers), equal
//! blocks keep id and layers, changed ones get text edits plus formatting marks from the file,
//! new / removed ones become `InsertBlock` / `RemoveBlock`.

use super::diff::segments;
use super::plan::block_text_ops;
use crate::doc::{Block, Document};
use crate::format::Exporter;
use crate::mark::Mark;
use crate::ops::{Command, Op};

pub(super) fn plan_blocks(doc: &Document, new_doc: &Document, exporter: &dyn Exporter) -> Vec<Op> {
    let key = |b: &Block| exporter.export_blocks(std::slice::from_ref(b));
    let ok: Vec<String> = doc.blocks.iter().map(key).collect();
    let nk: Vec<String> = new_doc.blocks.iter().map(key).collect();
    let mut work = doc.clone();
    let mut ops: Vec<Op> = Vec::new();
    let mut pos = 0usize;
    let mut push = |work: &mut Document, op: Op| {
        op.apply(work).expect("planned op is valid");
        ops.push(op);
    };
    for g in segments(&ok, &nk) {
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
