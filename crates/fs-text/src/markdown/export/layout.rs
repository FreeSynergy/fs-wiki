//! Placing a rendered block inside its containers.
//!
//! Pattern: Decorator-like prefixing — every line of the block gets the container prefixes;
//! the first line of a fresh container gets its marker, new callouts get a header line.

use super::callout::callout_header;
use super::container::{common, cont_prefix, marker};
use super::leaf::render_block;
use super::options::ExportOptions;
use crate::doc::{Block, Container};

pub(super) fn render_in_containers(
    b: &Block,
    prev: &[Container],
    out: &mut Vec<String>,
    opts: ExportOptions,
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
        if line.is_empty() {
            out.push(l.trim_end().to_string());
        } else {
            l.push_str(&line);
            out.push(l);
        }
        fresh.iter_mut().for_each(|f| *f = false);
    }
}
