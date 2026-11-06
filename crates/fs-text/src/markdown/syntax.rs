//! How each mark kind is written in Markdown.
//!
//! Pattern: Registry + Strategy — [`md_syntax`] maps a kind to its [`MdSyntax`]; the enum
//! implements [`MarkWriter`] (open / close strings). The mark core stays format-free.

use crate::mark::{Mark, MarkKind};

/// How a kind is written in Markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MdSyntax {
    /// `open text close`
    Delim(&'static str, &'static str),
    /// `[text](dest "title")`, `[[path|text]]`, `<url>`
    Link,
    /// `![alt](src)`
    Image,
    /// Verbatim.
    Raw,
    /// `[^label]`
    FootnoteRef,
    /// Only with `layers` export: attribute syntax (`{{name}}`, `[text]{#name}`).
    Layer,
}

/// The Markdown syntax of `kind`.
#[must_use]
pub fn md_syntax(kind: MarkKind) -> MdSyntax {
    match kind {
        MarkKind::Strong => MdSyntax::Delim("**", "**"),
        MarkKind::Emphasis => MdSyntax::Delim("*", "*"),
        MarkKind::Code => MdSyntax::Delim("`", "`"),
        MarkKind::Strike => MdSyntax::Delim("~~", "~~"),
        MarkKind::Link => MdSyntax::Link,
        MarkKind::Image => MdSyntax::Image,
        MarkKind::Html => MdSyntax::Raw,
        MarkKind::Passage | MarkKind::Variable | MarkKind::Explanation => MdSyntax::Layer,
        MarkKind::FootnoteRef => MdSyntax::FootnoteRef,
    }
}

/// Writes the opening and closing text of a mark.
pub trait MarkWriter {
    /// Text before the marked range. `delim` overrides a delimiter (`_` style) when non-empty.
    fn open(&self, delim: &str) -> String;
    /// Text after the marked range.
    fn close(&self, mark: &Mark, delim: &str) -> String;
}
