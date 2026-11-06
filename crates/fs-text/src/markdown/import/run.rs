//! The import entry point and event loop.
//!
//! Pattern: Facade — `import_markdown` hides the pre-pass (admonitions, loose lists), the
//! event loop and the attaching of source texts behind one function.

use super::admonition::convert_marked;
use super::loose::loose_lists;
use super::options::options;
use super::state::Imp;
use crate::doc::Document;
use crate::id::new_id;
use pulldown_cmark::{Event, Parser};
use std::ops::Range;

/// Import Markdown text as a block document. Every block remembers the source text it came
/// from (`BlockAttrs::source`), so an unchanged block can be written back byte for byte.
#[must_use]
pub fn import_markdown(orig: &str) -> Document {
    import_with(orig, true)
}

/// Import without keeping source texts (for the exporter's read-back checks).
pub(crate) fn import_plain(text: &str) -> Document {
    import_with(text, false)
}

fn import_with(orig: &str, sources: bool) -> Document {
    // a lone CR ends a line too (same length, so offsets and line numbers stay valid)
    let text = orig.replace("\r\n", "\n").replace('\r', "\n");
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let (src, headers) = convert_marked(text);
    let parser = Parser::new_ext(&src, options());
    let mut defs: Vec<Range<usize>> = parser
        .reference_definitions()
        .iter()
        .map(|(_, d)| d.span.clone())
        .collect();
    defs.sort_by_key(|r| r.start);
    let events: Vec<(Event, Range<usize>)> = parser.into_offset_iter().collect();
    let mut s = Imp::new(&src, loose_lists(&src, &events), defs, &headers);
    for (idx, (ev, range)) in events.iter().enumerate() {
        s.definitions_before(ev, range);
        s.event(idx, ev, range);
    }
    s.finish();
    let mut blocks = std::mem::take(&mut s.blocks);
    s.add_stray_definitions(&mut blocks);
    s.settle_glue(&mut blocks);
    if sources {
        s.attach_sources(&mut blocks, orig);
    }
    Document {
        id: new_id(),
        blocks,
    }
}
