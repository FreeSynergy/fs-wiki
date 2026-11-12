//! The reader's view of pages with language variants.
//!
//! Pattern: Adapter — the texts `en/x.md` and `de/x.md` are the variants of ONE page `docs/x.md`: the sidebar lists the page
//! once, a link to it opens it in the language the person chose, `docs/x.md@de` opens that variant, a language bar at the top
//! switches, and a page that lacks the chosen language is shown in another one with a notice (`lang_words`). The grouping is
//! `variants::PageIndex`; nothing here writes.

use std::collections::HashMap;
use std::sync::Mutex;

use fs_components_widgets::{Block, Document, Inline, Mark, MarkKind};
use fs_components_workbenches::{Entry, SourceError};
use fs_wiki_core::TextEntry;

use super::wiki::WikiSource;
use crate::lang_words as words;
use crate::map::to_document;
use crate::variants::{PageIndex, PageRecord};

/// The pages with variants, and the language the person reads in.
pub struct PagesView {
    index: PageIndex,
    /// The text path of every variant -> the path of its page.
    by_text: HashMap<String, String>,
    lang: Mutex<String>,
}

impl PagesView {
    /// A view over `index`, reading in `lang` first.
    pub fn new(index: PageIndex, lang: &str) -> Self {
        let by_text = index
            .pages
            .values()
            .flat_map(|p| p.variants.values().map(|v| (v.path.clone(), p.path())))
            .collect();
        Self {
            index,
            by_text,
            lang: Mutex::new(lang.to_owned()),
        }
    }

    fn language(&self) -> String {
        self.lang
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn set_language(&self, lang: &str) {
        let mut current = self
            .lang
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        lang.clone_into(&mut current);
    }

    /// The path of the page a text belongs to; a text that is no variant is its own page.
    pub fn page_of(&self, text_path: &str) -> String {
        self.by_text
            .get(text_path)
            .cloned()
            .unwrap_or_else(|| text_path.to_owned())
    }

    /// The language to show of `page`: the wanted one, else the origin, else the first there is.
    fn pick(&self, page: &PageRecord, wanted: &str) -> Option<String> {
        let origin = self.index.origin_of(&page.group);
        [wanted, origin]
            .into_iter()
            .find(|l| page.variants.contains_key(*l))
            .map(str::to_owned)
            .or_else(|| page.variants.keys().next().cloned())
    }

    /// The text path that `page` (as `docs/x.md` or `docs/x.md@de`) shows now, and the language it is in.
    pub fn text_of(&self, page: &str) -> Option<(String, String)> {
        let (page, asked) = split(page);
        let record = self.index.pages.get(page)?;
        let lang = self.pick(record, asked.unwrap_or(&self.language()))?;
        Some((record.variants[&lang].path.clone(), lang))
    }

    /// The text path of the page `from` (a page path, maybe with a language), for resolving the links written on it.
    pub fn text_path(&self, from: &str) -> String {
        self.text_of(from)
            .map_or_else(|| from.to_owned(), |(t, _)| t)
    }

    /// `page@lang` as a link target; the language becomes the one read in from now on.
    pub fn switch(&self, from: &str, lang: &str) -> String {
        self.set_language(lang);
        format!("{}@{lang}", split(from).0)
    }

    /// The lowest rank of any variant of the page (the navigation ranks texts).
    fn rank(page: &PageRecord, rank: &dyn Fn(&str) -> Option<usize>) -> usize {
        page.variants
            .values()
            .filter_map(|v| rank(&v.path))
            .min()
            .unwrap_or(usize::MAX)
    }
}

/// `docs/x.md@de` is (`docs/x.md`, `Some("de")`).
pub fn split(path: &str) -> (&str, Option<&str>) {
    match path.rsplit_once('@') {
        Some((page, lang))
            if !lang.is_empty() && lang.chars().all(|c| c.is_ascii_lowercase() || c == '-') =>
        {
            (page, Some(lang))
        }
        _ => (path, None),
    }
}

impl WikiSource {
    /// The sidebar entries when pages have variants: one per page, in the order of the navigation.
    pub(super) fn page_entries(
        &self,
        view: &PagesView,
        texts: &[TextEntry],
        prefix: &str,
    ) -> Vec<Entry> {
        let wanted = view.language();
        let names: HashMap<&str, &str> = texts
            .iter()
            .map(|t| (t.path.as_str(), t.envelope.name.as_str()))
            .collect();
        let mut seen = std::collections::HashSet::new();
        let mut out: Vec<(usize, Entry)> = Vec::new();
        for t in texts {
            let page = view.page_of(&t.path);
            if !page.starts_with(prefix) || !seen.insert(page.clone()) {
                continue;
            }
            let record = view.index.pages.get(&page);
            let title = record
                .and_then(|r| view.pick(r, &wanted).map(|l| &r.variants[&l].path))
                .and_then(|p| names.get(p.as_str()))
                .map_or(t.envelope.name.as_str(), |n| *n);
            let rank = record.map_or_else(
                || self.nav.rank(&t.path).unwrap_or(usize::MAX),
                |r| PagesView::rank(r, &|p| self.nav.rank(p)),
            );
            out.push((rank, Entry::new(page, title)));
        }
        out.sort_by_key(|(rank, _)| *rank);
        out.into_iter().map(|(_, e)| e).collect()
    }

    /// The page `path` in the language asked for (or the fallback), with the language bar and the notices on top; `None`
    /// when `path` is not a page with variants.
    pub(super) fn read_page(
        &self,
        view: &PagesView,
        path: &str,
    ) -> Result<Option<Document>, SourceError> {
        let (page, asked) = split(path);
        let Some(record) = view.index.pages.get(page) else {
            return Ok(None);
        };
        let ui = asked.map_or_else(|| view.language(), str::to_owned);
        let Some((text, shown)) = view.text_of(path) else {
            return Ok(None);
        };
        let entry = self.entry(&text)?;
        let mut doc = to_document(&entry.envelope.name, &entry.document);
        let mut top = Vec::new();
        if record.variants.len() > 1 {
            top.push(language_bar(&ui, &shown, record));
        }
        if shown != ui {
            top.push(notice(&words::only_in(&ui, &shown)));
        }
        if view.index.outdated(record, &shown) {
            top.push(notice(&words::outdated(
                &ui,
                view.index.origin_of(&record.group),
            )));
        }
        top.append(&mut doc.blocks);
        doc.blocks = top;
        Ok(Some(doc.with_revision(entry.revision.to_string())))
    }
}

/// `Language: Deutsch | English`, the others as links (`?lang=xx`, resolved by the source), the shown one plain and bold.
fn language_bar(ui: &str, shown: &str, record: &PageRecord) -> Block {
    let mut text = format!("{}: ", words::language_label(ui));
    let mut marks = Vec::new();
    for (i, lang) in record.variants.keys().enumerate() {
        if i > 0 {
            text.push_str(" | ");
        }
        let name = words::language_name(lang);
        let (start, end) = (
            text.chars().count(),
            text.chars().count() + name.chars().count(),
        );
        let kind = if lang == shown {
            MarkKind::Strong
        } else {
            MarkKind::Link {
                href: format!("?lang={lang}"),
            }
        };
        marks.push(Mark::new(kind, start, end));
        text.push_str(&name);
    }
    Block::Paragraph(Inline::new(text, marks))
}

/// A note above the page, set off as a quotation.
fn notice(text: &str) -> Block {
    Block::Quote(vec![Block::Paragraph(Inline::plain(text))])
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn a_language_is_split_off_a_page_path() {
        assert_eq!(split("docs/a.md@de"), ("docs/a.md", Some("de")));
        assert_eq!(split("docs/a.md"), ("docs/a.md", None));
        assert_eq!(split("docs/me@example.md"), ("docs/me@example.md", None));
    }
}
