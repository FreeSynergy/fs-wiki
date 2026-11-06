//! Changes from outside (ADR 0042 §5): an exported file edited elsewhere is an **import**.
//! The new file text is compared with the stored state; the differences become ordinary
//! edit ops, so marks move with the text. The caller applies the returned ops through
//! [`crate::History::apply`] (undoable) or [`Document::apply`].

use crate::doc::*;
use crate::mark::*;
use crate::md_export::{export_blocks, ExportOptions};
use crate::md_import::import_markdown;
use crate::ops::Op;
use similar::{capture_diff_slices, Algorithm, DiffOp};

/// One diff segment with positions derived from running counters (the index fields of
/// `similar`'s Insert/Delete ops are not reliable for our purposes).
#[derive(Clone, Copy, Debug)]
struct Seg {
    oi: usize,
    ol: usize,
    ni: usize,
    nl: usize,
    equal: bool,
}

fn segments<T: std::hash::Hash + Eq + Ord>(old: &[T], new: &[T]) -> Vec<Seg> {
    let (mut oi, mut ni) = (0, 0);
    let mut v = Vec::new();
    for d in capture_diff_slices(Algorithm::Myers, old, new) {
        let (ol, nl, equal) = match d {
            DiffOp::Equal { len, .. } => (len, len, true),
            DiffOp::Delete { old_len, .. } => (old_len, 0, false),
            DiffOp::Insert { new_len, .. } => (0, new_len, false),
            DiffOp::Replace { old_len, new_len, .. } => (old_len, new_len, false),
        };
        v.push(Seg { oi, ol, ni, nl, equal });
        oi += ol;
        ni += nl;
    }
    v
}

/// A text change in char offsets of the *old* text: replace `start..end` by `insert`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub start: usize,
    pub end: usize,
    pub insert: String,
}

/// Minimal edits turning `old` into `new`: a line diff first, then a char diff inside
/// changed hunks. Sorted ascending, non-overlapping.
pub fn diff_edits(old: &str, new: &str) -> Vec<TextEdit> {
    if old == new {
        return Vec::new();
    }
    let ol: Vec<&str> = old.split_inclusive('\n').collect();
    let nl: Vec<&str> = new.split_inclusive('\n').collect();
    let offs = |lines: &[&str]| -> Vec<usize> {
        let mut v = Vec::with_capacity(lines.len() + 1);
        let mut o = 0;
        v.push(0);
        for l in lines {
            o += l.chars().count();
            v.push(o);
        }
        v
    };
    let oo = offs(&ol);
    let mut edits = Vec::new();
    for g in segments(&ol, &nl) {
        if g.equal {
            continue;
        }
        let a: String = ol[g.oi..g.oi + g.ol].concat();
        let b: String = nl[g.ni..g.ni + g.nl].concat();
        let base = oo[g.oi];
        if g.ol == 0 || g.nl == 0 {
            edits.push(TextEdit { start: base, end: base + a.chars().count(), insert: b });
            continue;
        }
        let ac: Vec<char> = a.chars().collect();
        let bc: Vec<char> = b.chars().collect();
        for c in segments(&ac, &bc) {
            if !c.equal {
                edits.push(TextEdit { start: base + c.oi, end: base + c.oi + c.ol, insert: bc[c.ni..c.ni + c.nl].iter().collect() });
            }
        }
    }
    // coalesce touching edits: applying back to front needs non-overlapping, unambiguous ranges
    let mut merged: Vec<TextEdit> = Vec::with_capacity(edits.len());
    for e in edits {
        match merged.last_mut() {
            Some(last) if e.start <= last.end => {
                last.end = last.end.max(e.end);
                last.insert.push_str(&e.insert);
            }
            _ => merged.push(e),
        }
    }
    merged
}

/// Ops (applied back to front, so each offset is valid) for one block text change.
fn block_text_ops(block: uuid::Uuid, old: &str, new: &str) -> Vec<Op> {
    diff_edits(old, new).into_iter().rev().map(|e| Op::Edit { block, start: e.start, end: e.end, insert: e.insert, restore: None }).collect()
}

/// Plan the import of `new_text` into `doc` without changing `doc`.
/// * one-block raw documents (code / file mode): a text diff, layer marks move along;
/// * block documents: `new_text` is read as Markdown and aligned block by block.
pub fn plan_external_import(doc: &Document, new_text: &str) -> Vec<Op> {
    match doc.raw_text() {
        Some(old) => block_text_ops(doc.blocks[0].id, &old, new_text),
        None => plan_markdown(doc, new_text),
    }
}

fn plan_markdown(doc: &Document, new_md: &str) -> Vec<Op> {
    let new_doc = import_markdown(new_md);
    let key = |b: &Block| export_blocks(std::slice::from_ref(b), ExportOptions::default());
    let ok: Vec<String> = doc.blocks.iter().map(key).collect();
    let nk: Vec<String> = new_doc.blocks.iter().map(key).collect();
    let mut work = doc.clone();
    let mut ops: Vec<Op> = Vec::new();
    let mut pos = 0usize;
    let mut push = |work: &mut Document, op: Op| {
        work.apply(&op).expect("planned op is valid");
        ops.push(op);
    };
    for g in segments(&ok, &nk) {
        let (ol, ni, nl, equal) = (g.ol, g.ni, g.nl, g.equal);
        let paired = ol.min(nl);
        for k in 0..paired {
            let id = work.blocks[pos].id;
            let nb = &new_doc.blocks[ni + k];
            if !equal {
                for op in block_text_ops(id, &work.blocks[pos].text.to_string(), &nb.text.to_string()) {
                    push(&mut work, op);
                }
                // formatting comes from the file, layers stay
                let cur = &work.blocks[pos];
                let mut marks: Vec<Mark> = cur.marks.iter().filter(|m| !m.kind.is_markdown_native()).cloned().collect();
                marks.extend(nb.marks.iter().cloned());
                let old_native: Vec<&Mark> = cur.marks.iter().filter(|m| m.kind.is_markdown_native()).collect();
                let same = old_native.len() == nb.marks.len() && old_native.iter().zip(nb.marks.iter()).all(|(a, b)| a.kind == b.kind && a.start == b.start && a.end == b.end && a.value == b.value);
                if !same {
                    push(&mut work, Op::SetMarks { block: id, marks });
                }
            }
            if work.blocks[pos].attrs != nb.attrs {
                push(&mut work, Op::SetAttrs { block: id, attrs: nb.attrs.clone() });
            }
            pos += 1;
        }
        for _ in paired..ol {
            let id = work.blocks[pos].id;
            push(&mut work, Op::RemoveBlock { block: id });
        }
        for k in paired..nl {
            let nb = new_doc.blocks[ni + k].clone();
            push(&mut work, Op::InsertBlock { index: pos, block: nb });
            pos += 1;
        }
    }
    ops
}

/// Plan, apply and return `(forward ops, inverse ops in undo order)`.
pub fn import_external(doc: &mut Document, new_text: &str) -> (Vec<Op>, Vec<Op>) {
    let ops = plan_external_import(doc, new_text);
    let mut inv: Vec<Op> = ops.iter().map(|o| doc.apply(o).expect("planned op is valid")).collect();
    inv.reverse();
    (ops, inv)
}
