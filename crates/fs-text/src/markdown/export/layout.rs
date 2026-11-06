//! Placing a rendered block inside its containers.
//!
//! Pattern: Decorator-like prefixing — every line of the block gets the container prefixes;
//! the first line of a fresh container gets its marker, new callouts get a header line.

use super::callout::callout_header;
use super::container::{common, cont_prefix, marker};
use super::leaf::render_block;
use super::options::ExportOptions;
use crate::doc::{Block, BlockKind, Container};

/// Where a block stands, for the few forms that depend on the neighbourhood.
#[derive(Clone, Copy)]
pub(super) struct Place {
    /// First block of the document (`---` there could open front matter).
    pub first: bool,
    /// Directly under a paragraph line (`---` there would make a heading).
    pub under_paragraph: bool,
}

/// A thematic break that is not read as part of the prefix or as something else: `- ---` is one
/// rule and no item, `---` under a paragraph is a heading, `---` at the start may be front
/// matter. The style of the source is kept when it is safe, otherwise a character that is
/// not in the prefix is used.
fn rule_line(hint: Option<&str>, prefix: &str, place: Place) -> String {
    let safe =
        |c: char| !prefix.contains(c) && !(c == '-' && (place.first || place.under_paragraph));
    if let Some(h) = hint {
        if h.chars().next().is_some_and(safe) {
            return h.to_string();
        }
    }
    ['-', '*', '_']
        .into_iter()
        .find(|c| safe(*c))
        .map_or("***".into(), |c| c.to_string().repeat(3))
}

pub(super) fn render_in_containers(
    b: &Block,
    prev: &[Container],
    out: &mut Vec<String>,
    opts: ExportOptions,
    place: Place,
) {
    let cs = &b.attrs.containers;
    let k = common(prev, cs);
    let mut fresh: Vec<bool> = (0..cs.len()).map(|d| d >= k).collect();
    let prefix = |fresh: &[bool], upto: usize| -> String {
        cs[..upto]
            .iter()
            .enumerate()
            .map(|(d, c)| {
                if fresh[d] {
                    marker(c, opts)
                } else {
                    cont_prefix(c, opts)
                }
            })
            .collect()
    };
    for (d, c) in cs.iter().enumerate().skip(k) {
        if let Container::Callout {
            ty, title, fold, ..
        } = c
        {
            out.push(callout_header(prefix(&fresh, d), ty, title, *fold, opts));
            for f in fresh.iter_mut().take(d + 1) {
                *f = false;
            }
        }
    }
    if b.attrs.extra.contains_key("callout-header") && b.text.is_empty() {
        return;
    }
    for line in render_block(b, opts) {
        let mut l = prefix(&fresh, cs.len());
        let line = if b.attrs.kind == BlockKind::ThematicBreak {
            rule_line(b.attrs.extra.get("rule").map(String::as_str), &l, place)
        } else {
            line
        };
        if line.is_empty() {
            out.push(l.trim_end().to_string());
        } else {
            l.push_str(&line);
            out.push(l);
        }
        fresh.fill(false);
    }
}
