//! Mark payloads: link values and the value enum.
//!
//! Pattern: Newtype / value objects — small serialisable data types with their own rules
//! (`LinkValue::split_fragment`, `is_internal`).

use crate::id::Uuid;
use serde::{Deserialize, Serialize};

/// How a link was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkStyle {
    /// `[text](dest)`
    Inline,
    /// `<https://…>`
    Autolink,
    /// `[[path]]` / `[[path|label]]`
    Wiki,
    /// `![[path]]`
    Embed,
}

/// Target of a link or image mark.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkValue {
    /// Resolved target (page / block) once known; `None` for external or unresolved links.
    pub target: Option<Uuid>,
    /// Readable path or URL, exactly as written in the text layer.
    pub path: String,
    /// Optional link title.
    pub title: Option<String>,
    /// Written style.
    pub style: LinkStyle,
}

impl LinkValue {
    /// Unresolved link to `path`.
    #[must_use]
    pub fn new(path: impl Into<String>, style: LinkStyle) -> Self {
        Self {
            target: None,
            path: path.into(),
            title: None,
            style,
        }
    }
    /// `(page, fragment)` split at the first `#`.
    #[must_use]
    pub fn split_fragment(&self) -> (&str, Option<&str>) {
        match self.path.split_once('#') {
            Some((p, f)) => (p, Some(f)),
            None => (&self.path, None),
        }
    }
    /// Does the path point to an internal page (not an URL / mail)?
    #[must_use]
    pub fn is_internal(&self) -> bool {
        matches!(self.style, LinkStyle::Wiki | LinkStyle::Embed)
            || (!self.path.contains("://")
                && !self.path.starts_with("mailto:")
                && !self.path.starts_with('#'))
    }
}

/// Payload of a mark.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkValue {
    /// No payload.
    None,
    /// Link / image target.
    Link(LinkValue),
    /// Passage name, footnote label.
    Name(String),
    /// Variable placeholder.
    Variable {
        /// Variable name.
        name: String,
        /// Optional declared type.
        ty: Option<String>,
    },
    /// Own explanation text.
    Explanation {
        /// Replacement text.
        replacement: String,
    },
}
