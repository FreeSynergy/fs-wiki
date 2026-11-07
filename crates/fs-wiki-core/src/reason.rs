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
        })
    }
}
