//! Document-level export: block sequencing and blank lines.
//!
//! Pattern: Template method — decides per block whether a blank line separates it from the
//! previous one (tight lists, glued blocks) and delegates the rendering.

use super::container::{common, cont_prefix, tight_join};
use super::layout::render_in_containers;
use super::options::ExportOptions;
use crate::doc::{Block, BlockKind, Container, Document};

/// Export a document. A one-block [`BlockKind::Raw`] document exports its text verbatim.
#[must_use]
pub fn export_markdown(doc: &Document, opts: ExportOptions) -> String {
    export_blocks(&doc.blocks, opts)
}

/// Export a block sequence.
#[must_use]
pub fn export_blocks(blocks: &[Block], opts: ExportOptions) -> String {
    let mut out: Vec<String> = Vec::new();
    if let [b] = blocks {
        if b.attrs.kind == BlockKind::Raw {
            return b.text.to_string();
        }
    }
    let mut prev: &[Container] = &[];
    for (bi, b) in blocks.iter().enumerate() {
        let cs = &b.attrs.containers[..];
        if bi > 0 {
            let k = common(prev, cs);
            let tight = tight_join(prev, cs, k);
            let two_paras = blocks[bi - 1].attrs.kind == BlockKind::Paragraph
                && b.attrs.kind == BlockKind::Paragraph
                && k == cs.len()
                && prev.len() == cs.len();
            let glued = b.attrs.extra.contains_key("glue") && !two_paras;
            if !(tight || glued) {
                let pfx: String = cs[..k].iter().map(|c| cont_prefix(c, opts)).collect();
                out.push(pfx.trim_end().to_string());
            }
        }
        render_in_containers(b, prev, &mut out, opts);
        prev = cs;
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}
