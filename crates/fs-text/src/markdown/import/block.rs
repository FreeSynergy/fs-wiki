//! Leaf blocks: paragraph, heading, code, HTML, rule.
//!
//! Pattern: Builder step — each handler opens a [`Leaf`] of the right kind; text and inline
//! events fill it until the end event flushes it.

use super::hints::glue_before;
use super::leaf::Leaf;
use super::state::Imp;
use crate::doc::BlockKind;
use pulldown_cmark::{CodeBlockKind, HeadingLevel, Tag};
use std::ops::Range;

fn heading_level(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// `{#id .class key=value}` as written, if the heading has attributes.
fn heading_attrs(
    id: Option<&str>,
    classes: &[impl AsRef<str>],
    attrs: &[(impl AsRef<str>, Option<impl AsRef<str>>)],
) -> Option<String> {
    if id.is_none() && classes.is_empty() && attrs.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some(id) = id {
        parts.push(format!("#{id}"));
    }
    for c in classes {
        parts.push(format!(".{}", c.as_ref()));
    }
    for (k, v) in attrs {
        parts.push(match v {
            Some(v) => format!("{}={}", k.as_ref(), v.as_ref()),
            None => k.as_ref().to_string(),
        });
    }
    Some(format!("{{{}}}", parts.join(" ")))
}

impl Imp<'_> {
    pub(super) fn start_leaf_block(&mut self, tag: &Tag) {
        self.flush();
        let leaf = match tag {
            Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => {
                let mut l = Leaf::new(BlockKind::Heading {
                    level: heading_level(*level),
                });
                l.heading_attrs = heading_attrs(id.as_deref(), classes, attrs);
                l
            }
            Tag::CodeBlock(k) => {
                let info = match k {
                    CodeBlockKind::Fenced(i) => i.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                Leaf::new(BlockKind::Code { info })
            }
            Tag::HtmlBlock => Leaf::new(BlockKind::Html),
            _ => Leaf::new(BlockKind::Paragraph),
        };
        self.leaf = Some(leaf);
    }

    pub(super) fn rule(&mut self, range: &Range<usize>) {
        self.flush();
        let mut l = Leaf::new(BlockKind::ThematicBreak);
        l.glue = glue_before(self.src, range.start);
        l.start = Some(range.start);
        l.end = range.end;
        let style = self.src[range.clone()].trim();
        if style != "---" {
            l.rule = Some(style.to_string());
        }
        self.leaf = Some(l);
        self.flush();
    }
}
