//! Planning an external import: which ops turn the stored document into the new file.
//!
//! Pattern: Template method — raw documents are diffed as text, block documents are parsed
//! with an [`Importer`] and aligned (`blocks.rs`); both yield ops that the caller applies.

use super::blocks::plan_blocks;
use super::diff::diff_edits;
use crate::doc::Document;
use crate::format::{Exporter, Importer};
use crate::markdown::{MarkdownExporter, MarkdownImporter};
use crate::ops::{Command, Op};

/// Ops (applied back to front, so each offset is valid) for one block text change.
pub(super) fn block_text_ops(block: crate::id::Uuid, old: &str, new: &str) -> Vec<Op> {
    diff_edits(old, new)
        .into_iter()
        .rev()
        .map(|e| Op::Edit {
            block,
            start: e.start,
            end: e.end,
            insert: e.insert,
            restore: None,
        })
        .collect()
}

/// Plan the import of `new_text` into `doc` without changing `doc` (Markdown block files).
/// * one-block raw documents (code / file mode): a text diff, layer marks move along;
/// * block documents: `new_text` is read as Markdown and aligned block by block.
#[must_use]
pub fn plan_external_import(doc: &Document, new_text: &str) -> Vec<Op> {
    plan_external_import_with(
        doc,
        new_text,
        &MarkdownImporter,
        &MarkdownExporter::default(),
    )
}

/// Like [`plan_external_import`] for any format adapter.
#[must_use]
pub fn plan_external_import_with(
    doc: &Document,
    new_text: &str,
    importer: &dyn Importer,
    exporter: &dyn Exporter,
) -> Vec<Op> {
    match doc.raw_text() {
        Some(old) => block_text_ops(doc.blocks[0].id, &old, new_text),
        None => plan_blocks(doc, &importer.import(new_text), exporter),
    }
}

/// Plan, apply and return `(forward ops, inverse ops in undo order)`.
#[must_use]
pub fn import_external(doc: &mut Document, new_text: &str) -> (Vec<Op>, Vec<Op>) {
    let ops = plan_external_import(doc, new_text);
    let mut inv: Vec<Op> = ops
        .iter()
        .map(|o| o.apply(doc).expect("planned op is valid"))
        .collect();
    inv.reverse();
    (ops, inv)
}
