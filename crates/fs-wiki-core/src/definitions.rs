//! Link reference definitions in a block of text.
//!
//! Pattern: Specification — `[r]: dest "title"` is not a link mark: fs-text keeps it as an HTML
//! block marked `unparsed`. Only such a block holds definitions; text inside a `<pre>` or a
//! comment is just text, however much it looks like one. This file finds the destinations (as
//! char offsets) for the three users: the link index, the rewrite after a move, and the check
//! for comments.

use fs_text::{Block, BlockKind};

/// Where the destination of one definition stands in the block text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Target {
    /// First char of the destination (inside `<...>` if it is wrapped).
    pub start: usize,
    /// Chars in the destination.
    pub len: usize,
    /// It is written `<dest>`.
    pub angled: bool,
}

/// Whether `block` is made of link reference definitions the Markdown parser did not turn into
/// anything else.
pub(crate) fn is_definition_block(block: &Block) -> bool {
    block.attrs.kind == BlockKind::Html && block.attrs.extra.contains_key("unparsed")
}

fn skip_blanks(chars: &[char], mut at: usize) -> usize {
    while chars.get(at).is_some_and(|c| *c == ' ' || *c == '\t') {
        at += 1;
    }
    at
}

/// The destinations of the definitions in `block` (none if it is not a definition block). A
/// destination may stand on the line after its label.
pub(crate) fn targets(block: &Block) -> Vec<Target> {
    if !is_definition_block(block) {
        return vec![];
    }
    let chars: Vec<char> = block.text.to_string().chars().collect();
    let mut found = vec![];
    let mut line = 0;
    while line < chars.len() {
        let next = chars[line..]
            .iter()
            .position(|c| *c == '\n')
            .map_or(chars.len(), |n| line + n + 1);
        let lead = chars[line..next].iter().take_while(|c| **c == ' ').count();
        let open = line + lead;
        if lead <= 3 && chars.get(open) == Some(&'[') {
            let label =
                (open..next.saturating_sub(1)).find(|&i| chars[i] == ']' && chars[i + 1] == ':');
            if let Some(close) = label {
                let mut at = skip_blanks(&chars, close + 2);
                if chars.get(at) == Some(&'\n') {
                    at = skip_blanks(&chars, at + 1);
                }
                let angled = chars.get(at) == Some(&'<');
                let start = at + usize::from(angled);
                let len = chars[start.min(chars.len())..]
                    .iter()
                    .take_while(|c| {
                        if angled {
                            !matches!(**c, '>' | '\n')
                        } else {
                            !c.is_whitespace()
                        }
                    })
                    .count();
                if len > 0 || angled {
                    found.push(Target { start, len, angled });
                }
                line = (start + len).max(next).min(chars.len().max(next));
                // Continue after the line the destination ended on.
                line = chars[line.min(chars.len())..]
                    .iter()
                    .position(|c| *c == '\n')
                    .map_or(chars.len(), |n| line + n + 1);
                continue;
            }
        }
        line = next;
    }
    found
}

/// The destination text of `target` in `block`.
pub(crate) fn destination(block: &Block, target: &Target) -> String {
    block
        .text
        .to_string()
        .chars()
        .skip(target.start)
        .take(target.len)
        .collect()
}
