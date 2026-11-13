//! The steps of a picture of the wiki window (`--snapshot` with `--step`).
//!
//! Pattern: Interpreter — a step is a word (`open:<path>`, `edit`, `type:<text>`, `save` ...) that the reader of
//! `fs-components` applies; two are the app's own and make a page as the menu Page does (`new[:<title>]`,
//! `subpage[:<title>]`): they call the same `PageMaker`, then open the page in the editor, so a picture (and a test) shows
//! what the person sees after choosing the entry.

use fs_components_workbenches::{DocumentSource, Reader, Step};

use crate::page_words::{create_error, page_words};
use crate::source::PageMaker;

/// The reader's steps for `words`, the pages of `new` and `subpage` made through `maker`.
pub(crate) fn steps(words: &[String], maker: &dyn PageMaker, german: bool) -> Vec<Step> {
    let title = page_words(german).title;
    let mut out = Vec::new();
    let mut open: Option<String> = None;
    for word in words {
        let made = match word.split_once(':').unwrap_or((word.as_str(), "")) {
            ("new", t) => Some((None, t)),
            ("subpage", t) => match &open {
                Some(parent) => Some((Some(parent.clone()), t)),
                None => {
                    eprintln!("fs-wiki-app: subpage needs a page shown first (open:<path>)");
                    continue;
                }
            },
            _ => None,
        };
        let Some((parent, t)) = made else {
            if let Some(step) = step(word) {
                if let Step::Open(p) = &step {
                    open = Some(p.clone());
                }
                out.push(step);
            }
            continue;
        };
        match maker.create_page(parent.as_deref(), if t.is_empty() { title } else { t }) {
            Ok(path) => {
                open = Some(path.clone());
                out.extend([Step::Open(path), Step::Edit]);
            }
            Err(e) => eprintln!("fs-wiki-app: {}", create_error(&e, german)),
        }
    }
    out
}

/// `open:<path>`, `follow:<href>`, `back`, `search:<term>`, `scroll:<n>`, `expand:<key>`, `edit`, `type:<text>`, `save`.
fn step(text: &str) -> Option<Step> {
    match text.split_once(':') {
        Some(("open", p)) => Some(Step::Open(p.into())),
        Some(("follow", h)) => Some(Step::Follow(h.into())),
        Some(("search", t)) => Some(Step::Search(t.into())),
        Some(("scroll", n)) => n.parse().ok().map(Step::Scroll),
        Some(("expand", k)) => Some(Step::Expand(k.into())),
        Some(("type", t)) => Some(Step::Type(t.replace("\\n", "\n"))),
        None if text == "edit" => Some(Step::Edit),
        None if text == "save" => Some(Step::Save),
        None if text == "cancel" => Some(Step::Cancel),
        None if text == "back" => Some(Step::Back),
        _ => {
            eprintln!("fs-wiki-app: ignoring unknown step {text:?}");
            None
        }
    }
}

/// What a picture reports on stderr besides its file, one line, for tests and for a person who wants words instead of a
/// picture: the page shown, whether it can be edited, the text of the edit in progress, the text the wiki holds for the page
/// shown, the notice and the problem of the reader, and the pages (path = title) in the order of the tree.
pub(crate) fn state_line(reader: &Reader, source: &dyn DocumentSource) -> String {
    let stored = reader
        .current_path()
        .and_then(|p| source.source_text(p).ok())
        .map(|t| t.text);
    let pages: Vec<String> = reader
        .entries()
        .iter()
        .map(|e| format!("{} = {}", e.path, e.title))
        .collect();
    format!(
        "state: path={:?} can_edit={} editing={:?} stored={:?} notice={:?} problem={:?} pages={:?}",
        reader.current_path(),
        reader.can_edit(),
        reader.editing().map(|e| e.draft.as_str()),
        stored,
        reader.notice(),
        reader.problem(),
        pages
    )
}
