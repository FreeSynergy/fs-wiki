//! Document-level export: block sequencing and blank lines.
//!
//! Pattern: Template method — decides per block whether it is written back verbatim from its
//! kept source (unchanged) or canonically, whether a blank line separates it from the previous
//! one (tight lists, glued blocks) and delegates the rendering.

use super::container::{common, cont_prefix, lists_would_merge, tight_join};
use super::keep::{flush, kept_lead, kept_tail, line_ending, open_html, opts_for};
use super::layout::{render_in_containers, Place};
use super::options::ExportOptions;
use crate::doc::{same_in_file, usable_source, Block, BlockKind, Container, Document};
use crate::markdown::import_plain;

/// Export a document. A one-block [`BlockKind::Raw`] document exports its text verbatim.
#[must_use]
pub fn export_markdown(doc: &Document, opts: ExportOptions) -> String {
    export_blocks(&doc.blocks, opts)
}

/// What one pass of writing did.
struct Pass {
    out: String,
    /// Per block: written from its kept source.
    verbatim: Vec<bool>,
    /// The last block was written from a source that reaches the end of the file.
    to_end: bool,
}

/// Export a block sequence. A block that still equals what it was read from (content, marks,
/// attributes, same predecessor) is written back from its kept source text; every other block
/// is written canonically. When anything was edited the result proves itself: it is read back
/// and must say what the model says. If not, the changed blocks and their neighbours are
/// written canonically, and if that still does not read back, the whole document is.
#[must_use]
pub fn export_blocks(blocks: &[Block], opts: ExportOptions) -> String {
    if blocks.is_empty() {
        return String::new();
    }
    if let [b] = blocks {
        if b.attrs.kind == BlockKind::Raw {
            return b.text.to_string();
        }
    }
    let n = blocks.len();
    let mut force = vec![opts.layers; n];
    let mut core: Vec<usize> = Vec::new();
    // radius 0 is the plain pass; then the changed blocks and their surroundings, ever wider
    let mut radius = 0usize;
    let mut fallback: Option<String> = None;
    loop {
        let pass = write(blocks, opts, &force, radius == 0);
        let kept = pass.verbatim.iter().any(|v| *v);
        let edited = !kept || pass.verbatim.iter().any(|v| !v) || !pass.to_end;
        if !kept || !edited {
            return pass.out;
        }
        let back = import_plain(&pass.out).blocks;
        if same_in_file(blocks, &back) {
            return pass.out;
        }
        // not proven: remember a pass whose only differences are inside the new blocks
        if radius > 0 && fallback.is_none() && confined(blocks, &back, &pass.verbatim) {
            fallback = Some(pass.out);
        }
        if radius == 0 {
            core = (0..n).filter(|i| !pass.verbatim[*i]).collect();
            if !pass.to_end {
                core.push(n - 1);
            }
        }
        radius = if radius == 0 { 1 } else { radius * 3 };
        if radius >= n {
            break;
        }
        force = vec![false; n];
        for &i in &core {
            force[i.saturating_sub(radius)..=(i + radius).min(n - 1)].fill(true);
        }
    }
    // the whole document written anew; if even that does not read back, the model has a block
    // the canonical writer cannot say, and the least invasive proven-around pass is kept
    let all = write(blocks, opts, &vec![true; n], false).out;
    match fallback {
        Some(f) if !same_in_file(blocks, &import_plain(&all).blocks) => f,
        _ => all,
    }
}

/// Does the file differ from the model only inside blocks that were written anew? Then the
/// kept sources are proven, and what is left is what the canonical writer cannot say better.
fn confined(model: &[Block], back: &[Block], verbatim: &[bool]) -> bool {
    if model.len() != back.len() {
        return false;
    }
    let mut patched = model.to_vec();
    for (i, kept) in verbatim.iter().enumerate() {
        if !kept {
            patched[i] = back[i].clone();
        }
    }
    same_in_file(&patched, back)
}

fn write(blocks: &[Block], opts: ExportOptions, force: &[bool], keep_layout: bool) -> Pass {
    let eol = line_ending(blocks);
    let mut res = String::new();
    let mut lines: Vec<String> = Vec::new();
    let mut prev: &[Container] = &[];
    let mut verbatim = vec![false; blocks.len()];
    let mut to_end = false;
    let mut bi = 0;
    while bi < blocks.len() {
        if !opts.layers && !force[bi] {
            if let Some((src, n)) =
                usable_source(blocks, bi).filter(|(_, n)| !force[bi..bi + n].contains(&true))
            {
                flush(&mut res, &mut lines, eol);
                if !res.is_empty() && !res.ends_with(['\n', '\r']) {
                    res.push_str(eol);
                }
                res.push_str(&src.text);
                prev = &blocks[bi + n - 1].attrs.containers;
                verbatim[bi..bi + n].fill(true);
                to_end = src.tail && bi + n == blocks.len();
                bi += n;
                continue;
            }
        }
        to_end = false;
        let b = &blocks[bi];
        let o = opts_for(b, opts);
        let cs = &b.attrs.containers[..];
        let mut separated = false;
        if bi > 0 {
            let k = common(prev, cs);
            let tight = tight_join(prev, cs, k);
            let two_paras = blocks[bi - 1].attrs.kind == BlockKind::Paragraph
                && b.attrs.kind == BlockKind::Paragraph
                && k == cs.len()
                && prev.len() == cs.len();
            // a line under an HTML block would belong to it
            let glued =
                b.attrs.extra.contains_key("glue") && !two_paras && !open_html(&blocks[bi - 1]);
            if !(tight || glued) {
                separated = true;
                let pfx: String = cs[..k].iter().map(|c| cont_prefix(c, o)).collect();
                if k < cs.len() && prev.get(k).is_some_and(|p| lists_would_merge(p, &cs[k])) {
                    // two lists of one type would read back as one: a comment keeps them apart
                    lines.push(pfx.trim_end().to_string());
                    lines.push(format!("{pfx}<!-- -->"));
                }
                match kept_lead(b, Some(&blocks[bi - 1])).filter(|_| pfx.is_empty() && keep_layout)
                {
                    Some(lead) => lines.extend(lead),
                    None => lines.push(pfx.trim_end().to_string()),
                }
            }
        }
        let place = Place {
            first: bi == 0,
            under_paragraph: bi > 0
                && !separated
                && blocks[bi - 1].attrs.kind == BlockKind::Paragraph,
        };
        render_in_containers(b, prev, &mut lines, o, place);
        if keep_layout && bi + 1 == blocks.len() {
            lines.extend(kept_tail(b));
        }
        prev = cs;
        bi += 1;
    }
    flush(&mut res, &mut lines, eol);
    if res.is_empty() {
        res.push_str(eol);
    }
    Pass {
        out: res,
        verbatim,
        to_end,
    }
}
