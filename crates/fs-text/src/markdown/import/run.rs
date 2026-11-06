//! The import entry point and event loop.
//!
//! Pattern: Facade — `import_markdown` hides the pre-pass (admonitions, loose lists) and the
//! event loop behind one function.

use super::admonition::convert_admonitions;
use super::loose::loose_lists;
use super::options::options;
use super::state::Imp;
use crate::doc::Document;
use crate::id::new_id;
use pulldown_cmark::{Event, Parser};
use std::ops::Range;

/// Import Markdown text as a block document.
#[must_use]
pub fn import_markdown(src: &str) -> Document {
    let src = convert_admonitions(&src.replace("\r\n", "\n"));
    let events: Vec<(Event, Range<usize>)> = Parser::new_ext(&src, options())
        .into_offset_iter()
        .collect();
    let mut s = Imp::new(&src, loose_lists(&src, &events));
    for (idx, (ev, range)) in events.iter().enumerate() {
        s.event(idx, ev, range);
    }
    s.finish();
    Document {
        id: new_id(),
        blocks: s.blocks,
    }
}
