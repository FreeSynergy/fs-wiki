//! The readable path of a text.
//!
//! Pattern: Newtype — a [`TextPath`] can only be built from a valid path, so the rest of the
//! crate never re-checks. A path is `segment/segment/...`: no empty segment, no `.` or `..`, no
//! `#` (that starts a fragment in a link), no backslash and no control characters. Case counts:
//! `Manual` and `manual` are two paths.

use crate::error::{Error, Result};
use crate::reason::Reason;

/// A valid readable path such as `manual/start`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextPath(String);

impl TextPath {
    /// Check `path`.
    ///
    /// # Errors
    /// [`Error::InvalidPath`] with the reason.
    pub fn parse(path: &str) -> Result<Self> {
        let bad = |reason| {
            Err(Error::InvalidPath {
                path: path.to_string(),
                reason,
            })
        };
        if path.is_empty() {
            return bad(Reason::Empty);
        }
        if path.contains('#') {
            return bad(Reason::Fragment);
        }
        if path.contains('\\') || path.chars().any(char::is_control) {
            return bad(Reason::Control);
        }
        if path.contains('\u{10FFFF}') {
            // It ends the text-order range that lists a folder, so such a text could not be listed.
            return bad(Reason::Reserved);
        }
        for segment in path.split('/') {
            match segment {
                "" => return bad(Reason::EmptySegment),
                "." | ".." => return bad(Reason::DotSegment),
                s if s.trim() != s => return bad(Reason::EdgeSpace),
                _ => {}
            }
        }
        Ok(Self(path.to_string()))
    }

    /// The path as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The last segment, e.g. `start` for `manual/start`.
    #[must_use]
    pub fn last_segment(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }
}

/// The page part of a link path: everything before `#`.
#[must_use]
pub fn page_of(link_path: &str) -> &str {
    link_path.split('#').next().unwrap_or(link_path)
}
