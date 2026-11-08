//! Why a path or a move is refused.
//!
//! Pattern: none (plain data). The sentence is for people (`Display`, unchanged from before); the
//! token is for programs and is part of the interface: a client reads it instead of the sentence.

use std::fmt;

/// The reason a path or a folder move is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The path is empty.
    Empty,
    /// The path contains `#`.
    Fragment,
    /// A backslash or a control character.
    Control,
    /// U+10FFFF, the end of the order range.
    Reserved,
    /// A leading, trailing or double `/`.
    EmptySegment,
    /// A `.` or `..` segment.
    DotSegment,
    /// A segment with leading or trailing space.
    EdgeSpace,
    /// A folder moved into itself.
    InsideItself,
    /// A moved text would land inside the old folder.
    InsideOld,
    /// A folder name that has no letter or digit a path can use.
    EmptySlug,
    /// A folder path that is not strictly below its parent's path.
    OutsideParent,
    /// A redirect target that is an outside address (`https:...`).
    ExternalTarget,
}

impl Reason {
    /// The short machine-readable form (`empty`, `fragment`, ...).
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Fragment => "fragment",
            Self::Control => "control-character",
            Self::Reserved => "reserved-character",
            Self::EmptySegment => "empty-segment",
            Self::DotSegment => "dot-segment",
            Self::EdgeSpace => "edge-space",
            Self::InsideItself => "inside-itself",
            Self::InsideOld => "inside-old-folder",
            Self::EmptySlug => "empty-slug",
            Self::OutsideParent => "outside-parent",
            Self::ExternalTarget => "external-target",
        }
    }
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "empty",
            Self::Fragment => "'#' starts a fragment, it is not part of a path",
            Self::Control => "backslash or control character",
            Self::Reserved => "U+10FFFF is reserved as the end of the order",
            Self::EmptySegment => "empty segment (leading, trailing or double '/')",
            Self::DotSegment => "'.' and '..' are not allowed",
            Self::EdgeSpace => "segment with leading or trailing space",
            Self::InsideItself => "the target lies inside the folder itself",
            Self::InsideOld => {
                "a moved text would land inside the old folder, which an interrupted move could not tell apart"
            }
            Self::EmptySlug => "the name has no letter or digit to make a path of; give a path",
            Self::OutsideParent => "the path does not lie below the path of the parent",
            Self::ExternalTarget => "a redirect leads to a path of this wiki, not to an outside address",
        })
    }
}
