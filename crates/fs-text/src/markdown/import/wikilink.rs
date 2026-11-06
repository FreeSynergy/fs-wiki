//! Links, images, autolinks and wiki links (`[[path|text]]`, `![[embed]]`).
//!
//! Pattern: Adapter — pulldown-cmark's link types are translated into our `LinkStyle` and
//! `LinkValue`, so the rest of the crate never sees parser types.

use super::state::Imp;
use crate::mark::{LinkStyle, LinkValue, MarkKind, MarkValue};
use pulldown_cmark::{LinkType, Tag};

fn style(link_type: LinkType, is_image: bool) -> LinkStyle {
    match link_type {
        LinkType::Autolink | LinkType::Email => LinkStyle::Autolink,
        LinkType::WikiLink { .. } if is_image => LinkStyle::Embed,
        LinkType::WikiLink { .. } => LinkStyle::Wiki,
        _ => LinkStyle::Inline,
    }
}

impl Imp<'_> {
    pub(super) fn start_link(&mut self, tag: &Tag) {
        let (Tag::Link {
            link_type,
            dest_url,
            title,
            ..
        }
        | Tag::Image {
            link_type,
            dest_url,
            title,
            ..
        }) = tag
        else {
            return;
        };
        let is_image = matches!(tag, Tag::Image { .. });
        self.ensure_leaf();
        let mut lv = LinkValue::new(dest_url.to_string(), style(*link_type, is_image));
        if !title.is_empty() {
            lv.title = Some(title.to_string());
        }
        self.open(
            if is_image {
                MarkKind::Image
            } else {
                MarkKind::Link
            },
            MarkValue::Link(lv),
        );
    }
}
