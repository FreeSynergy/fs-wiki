//! Parser options and format constants.
//!
//! Pattern: Configuration constants — one place that says which Markdown extensions exist.

use pulldown_cmark::Options;

pub(super) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_WIKILINKS
}

/// Hard line break is stored as U+2028 inside block text.
pub const HARD_BREAK: char = '\u{2028}';
