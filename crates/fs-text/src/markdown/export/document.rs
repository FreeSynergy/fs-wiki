//! Document-level export: block sequencing and blank lines.
//!
//! Pattern: Template method — decides per block whether it is written back verbatim from its
//! kept source (unchanged) or canonically, whether a blank line separates it from the previous
//! one (tight lists, glued blocks) and delegates the rendering.

use super::container::{common, cont_prefix, lists_would_merge, tight_join};
use super::layout::{render_in_containers, Place};
use super::options::ExportOptions;
use crate::doc::{usable_source, Block, BlockKind, Container, Document};

/// Export a document. A one-block [`BlockKind::Raw`] document exports its text verbatim.
#[must_use]
pub fn export_markdown(doc: &Document, opts: ExportOptions) -> String {
    export_blocks(&doc.blocks, opts)
}

/// Options for one block: a callout that was read as an admonition stays one.
fn opts_for(b: &Block, opts: ExportOptions) -> ExportOptions {
    ExportOptions {
        admonitions: opts.admonitions || b.attrs.extra.contains_key("admonition"),
        ..opts
    }
}

/// The line ending of the file the blocks were read from (`\r\n` if a kept source has one).
fn line_ending(blocks: &[Block]) -> &'static str {
    let crlf = blocks
        .iter()
        .filter_map(|b| b.attrs.source.as_ref())
        .any(|s| s.text.contains("\r\n"));
    if crlf {
        "\r\n"
    } else {
        "\n"
    }
}

/// A line with nothing but blanks and quote marks.
fn blankish(l: &str) -> bool {
    l.chars().all(|c| c.is_whitespace() || c == '>')
}

/// The blank lines in front of a changed block, as they were in the file, if the block still
/// follows the block it followed there (the layout around a changed block stays as it was).
fn kept_lead(b: &Block, prev: Option<&Block>) -> Option<Vec<String>> {
    let s = b.attrs.source.as_ref()?;
    if s.prev != prev.map(|p| p.id) {
        return None;
    }
    let lead: Vec<String> = s
        .text
        .lines()
        .take_while(|l| blankish(l))
        .map(String::from)
        .collect();
    (!lead.is_empty() && s.text.lines().count() > lead.len()).then_some(lead)
}

/// The blank lines that ended the file after a changed last block.
fn kept_tail(b: &Block) -> Vec<String> {
    let Some(s) = b.attrs.source.as_ref() else {
        return Vec::new();
    };
    let mut v: Vec<String> = s
        .text
        .lines()
        .rev()
        .take_while(|l| blankish(l))
        .map(String::from)
        .collect();
    v.reverse();
    if v.len() == s.text.lines().count() {
        v.clear();
    }
    v
}

/// Write the canonical lines gathered so far to `res`.
fn flush(res: &mut String, lines: &mut Vec<String>, eol: &str) {
    if lines.is_empty() {
        return;
    }
    if !res.is_empty() && !res.ends_with(['\n', '\r']) {
        res.push_str(eol);
    }
    // the kept source before already ends with a blank line: do not add a second one
    let ends_blank = res.ends_with("\n\n") || res.ends_with("\r\n\r\n");
    let skip = usize::from(ends_blank && lines[0].is_empty());
    for l in &lines[skip..] {
        res.push_str(l);
        res.push_str(eol);
    }
    lines.clear();
}

/// Export a block sequence. A block that still equals what it was read from (content, marks,
/// attributes, same predecessor) is written back from its kept source text; every other block
/// is written canonically.
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
    let eol = line_ending(blocks);
    let mut res = String::new();
    let mut lines: Vec<String> = Vec::new();
    let mut prev: &[Container] = &[];
    let mut bi = 0;
    while bi < blocks.len() {
        if !opts.layers {
            if let Some((src, n)) = usable_source(blocks, bi) {
                flush(&mut res, &mut lines, eol);
                if !res.is_empty() && !res.ends_with(['\n', '\r']) {
                    res.push_str(eol);
                }
                res.push_str(&src.text);
                prev = &blocks[bi + n - 1].attrs.containers;
                bi += n;
                continue;
            }
        }
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
            let glued = b.attrs.extra.contains_key("glue") && !two_paras;
            if !(tight || glued) {
                separated = true;
                let pfx: String = cs[..k].iter().map(|c| cont_prefix(c, o)).collect();
                if k < cs.len() && prev.get(k).is_some_and(|p| lists_would_merge(p, &cs[k])) {
                    // two lists of one type would read back as one: a comment keeps them apart
                    lines.push(pfx.trim_end().to_string());
                    lines.push(format!("{pfx}<!-- -->"));
                }
                match kept_lead(b, Some(&blocks[bi - 1])).filter(|_| pfx.is_empty()) {
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
        if bi + 1 == blocks.len() {
            lines.extend(kept_tail(b));
        }
        prev = cs;
        bi += 1;
    }
    flush(&mut res, &mut lines, eol);
    if res.is_empty() {
        res.push_str(eol);
    }
    res
}
