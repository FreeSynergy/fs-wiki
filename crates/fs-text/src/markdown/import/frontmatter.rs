//! YAML / TOML front matter.
//!
//! Pattern: Builder step — the fence style (`---` / `+++`) is remembered so export restores it.

use super::leaf::Leaf;
use super::state::Imp;
use crate::doc::BlockKind;
use pulldown_cmark::MetadataBlockKind;

impl Imp<'_> {
    pub(super) fn start_front_matter(&mut self, kind: MetadataBlockKind) {
        self.flush();
        let mut l = Leaf::new(BlockKind::FrontMatter);
        if matches!(kind, MetadataBlockKind::PlusesStyle) {
            l.heading_attrs = Some("+++".into());
        }
        self.leaf = Some(l);
    }
}
