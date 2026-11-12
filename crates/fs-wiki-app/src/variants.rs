//! Language variants of one page: DE and EN are ONE page with two variants.
//!
//! Pattern: Value objects + Repository over a JSON file — a page has an id (UUIDv7, made once), and per language a variant:
//! the text that holds it, its revision, and `source_rev`, the revision of the ORIGIN variant it was translated from. A
//! translation whose origin has another revision now is outdated. This is the page level of ADR 0072 (the block level
//! adds a block id per language with `translationOf` and the same `sourceRev` later, in the same words). The texts stay what
//! they are, one per variant and file (`en/<path>`, `de/<path>`), so the round trip is byte for byte and an export writes the
//! two files again; the grouping is this app's own record (`<records>.pages`), because the contract `texts.capnp` has no
//! field for it (named in `DESIGN.md`). Which language is the origin is set per group (default `en`; the decisions: `de`).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use fs_objects::ObjectId;
use fs_wiki_core::{Paging, Texts};
use serde::{Deserialize, Serialize};

/// A folder whose texts are the variants of the pages of `group`, in language `lang`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantSource {
    /// The path prefix of its texts (`en`).
    pub prefix: String,
    /// The family of pages it belongs to (`docs`); sources of one group with equal relative paths are one page.
    pub group: String,
    /// The language of its texts (`en`, `de`).
    pub lang: String,
}

/// One language of a page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    /// The text that holds it.
    pub text: ObjectId,
    /// Its path (`en/start/index.md`).
    pub path: String,
    /// The revision of the text when it was last linked.
    pub rev: u64,
    /// For a translation: the revision of the origin it was translated from; none for the origin itself.
    pub source_rev: Option<u64>,
}

/// A page: its id and its variants by language.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRecord {
    /// Made once, kept for ever.
    pub id: ObjectId,
    /// The family (`docs`).
    pub group: String,
    /// The path inside the family (`programs/index.md`).
    pub key: String,
    /// By language.
    pub variants: BTreeMap<String, Variant>,
}

impl PageRecord {
    /// The path the reader shows: `group/key`.
    pub fn path(&self) -> String {
        format!("{}/{}", self.group, self.key)
    }
}

/// All pages with variants.
#[derive(Clone, Debug, Default)]
pub struct PageIndex {
    /// The pages by their path (`group/key`).
    pub pages: BTreeMap<String, PageRecord>,
    /// The origin language of each group (default `en`).
    pub origin: HashMap<String, String>,
}

impl PageIndex {
    /// The origin language of `group`.
    pub fn origin_of(&self, group: &str) -> &str {
        self.origin.get(group).map_or("en", String::as_str)
    }

    /// Is the variant `lang` of `page` older than its origin?
    pub fn outdated(&self, page: &PageRecord, lang: &str) -> bool {
        let origin = page.variants.get(self.origin_of(&page.group));
        match (page.variants.get(lang), origin) {
            (Some(v), Some(o)) if v.source_rev.is_some() => v.source_rev != Some(o.rev),
            _ => false,
        }
    }

    /// The pages read from `file` (none when it does not exist yet).
    ///
    /// # Errors
    /// The file exists and does not read.
    pub fn load(file: &Path, origin: HashMap<String, String>) -> Result<Self, String> {
        let pages = match std::fs::read_to_string(file) {
            Ok(text) => serde_json::from_str::<Vec<PageRecord>>(&text)
                .map_err(|e| format!("{}: the pages do not read: {e}", file.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(format!("{}: {e}", file.display())),
        };
        Ok(Self {
            pages: pages.into_iter().map(|p| (p.path(), p)).collect(),
            origin,
        })
    }

    /// Write the pages to `file` (under a temporary name, then renamed over it).
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn save(&self, file: &Path) -> Result<(), String> {
        let list: Vec<&PageRecord> = self.pages.values().collect();
        let text = serde_json::to_string(&list).map_err(|e| e.to_string())?;
        let temp = file.with_extension("pages.tmp");
        std::fs::write(&temp, text)
            .and_then(|()| std::fs::rename(&temp, file))
            .map_err(|e| format!("{}: {e}", file.display()))
    }

    /// Bring the pages in step with the texts of `sources`: new pages get an id, a variant whose text changed since the last
    /// linking is "in step" with the origin as it is now, one that did not change keeps the revision it was translated from
    /// (so a changed origin makes it outdated).
    ///
    /// # Errors
    /// A source cannot be listed.
    pub async fn link(self, texts: &dyn Texts, sources: &[VariantSource]) -> Result<Self, String> {
        let mut heads = Vec::new();
        for s in sources {
            let below = format!("{}/", s.prefix);
            let list = texts
                .list(&below, Paging::default())
                .await
                .map_err(|e| format!("{}: {e}", s.prefix))?;
            heads.extend(list.into_iter().map(|t| crate::heads::Head {
                id: t.id,
                path: t.path,
                title: t.envelope.name,
                revision: t.revision,
            }));
        }
        Ok(self.link_heads(&heads, sources))
    }

    /// [`link`](Self::link) over heads already read (the file of `--index` or one list of everything).
    #[must_use]
    pub fn link_heads(mut self, heads: &[crate::heads::Head], sources: &[VariantSource]) -> Self {
        let mut found: BTreeMap<(String, String), BTreeMap<String, Variant>> = BTreeMap::new();
        for s in sources {
            let below = format!("{}/", s.prefix);
            for t in heads.iter().filter(|h| h.path.starts_with(&below)) {
                let key = t.path[below.len()..].to_string();
                found.entry((s.group.clone(), key)).or_default().insert(
                    s.lang.clone(),
                    Variant {
                        text: t.id,
                        path: t.path.clone(),
                        rev: t.revision,
                        source_rev: None,
                    },
                );
            }
        }
        let mut pages = BTreeMap::new();
        for ((group, key), mut variants) in found {
            let path = format!("{group}/{key}");
            let before = self.pages.remove(&path);
            let origin = variants.get(self.origin_of(&group)).map(|v| v.rev);
            for (lang, v) in &mut variants {
                if *lang == self.origin_of(&group) {
                    continue;
                }
                let old = before.as_ref().and_then(|p| p.variants.get(lang));
                v.source_rev = match old {
                    Some(o) if o.rev == v.rev && o.text == v.text => o.source_rev,
                    _ => origin,
                };
            }
            let id = before.map_or_else(ObjectId::new, |p| p.id);
            pages.insert(
                path,
                PageRecord {
                    id,
                    group,
                    key,
                    variants,
                },
            );
        }
        self.pages = pages;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(origin_rev: u64, source_rev: Option<u64>) -> (PageIndex, PageRecord) {
        let variant = |rev, source_rev| Variant {
            text: ObjectId::new(),
            path: "x".into(),
            rev,
            source_rev,
        };
        let record = PageRecord {
            id: ObjectId::new(),
            group: "docs".into(),
            key: "a.md".into(),
            variants: [
                ("en".to_owned(), variant(origin_rev, None)),
                ("de".to_owned(), variant(9, source_rev)),
            ]
            .into(),
        };
        (PageIndex::default(), record)
    }

    #[test]
    fn a_translation_is_outdated_when_its_origin_moved_on() {
        let (index, in_step) = page(5, Some(5));
        assert!(!index.outdated(&in_step, "de"));
        let (index, behind) = page(6, Some(5));
        assert!(index.outdated(&behind, "de"));
        assert!(
            !index.outdated(&behind, "en"),
            "the origin is never outdated"
        );
        let (index, unknown) = page(6, None);
        assert!(
            !index.outdated(&unknown, "de"),
            "no source revision, no claim"
        );
    }
}
