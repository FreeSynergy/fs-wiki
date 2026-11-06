//! Link reference definitions after a move.
//!
//! Pattern: Command producer — `[r]: dest "title"` is stored as a plain block (HTML or raw), not
//! as a link mark, so `relink.rs` handles the marks and this file the definitions: each
//! destination goes through the same rule as an inline link, and no definition keeps the old
//! target.

use fs_text::{Block, BlockKind, Document, LinkStyle, LinkValue, Op};

use super::relink::{new_written, Shift};

/// Link reference definitions (`[r]: dest "title"`, stored as plain blocks) get the new target
/// too, so none keeps the old one.
pub(super) fn relink_definitions(
    out: &mut Document,
    block: &Block,
    before: &str,
    after: &str,
    shift: &Shift,
) -> bool {
    if !matches!(block.attrs.kind, BlockKind::Raw | BlockKind::Html) {
        return false;
    }
    let mut edits = vec![];
    let mut offset = 0;
    for line in block.text.to_string().split_inclusive('\n') {
        if let Some((start, len)) = definition_target(line) {
            let dest: String = line.chars().skip(start).take(len).collect();
            let link = LinkValue::new(dest, LinkStyle::Inline);
            if let Some(new) = new_written(&link, before, after, shift) {
                edits.push((offset + start, offset + start + len, new));
            }
        }
        offset += line.chars().count();
    }
    let changed = !edits.is_empty();
    for (start, end, insert) in edits.into_iter().rev() {
        let op = Op::Edit {
            block: block.id,
            start,
            end,
            insert,
            restore: None,
        };
        out.apply(&op).expect("valid op");
    }
    changed
}

/// (start, length) in chars of the destination of a definition line `[label]: dest ...`.
fn definition_target(line: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = line.chars().collect();
    let lead = chars.iter().take_while(|c| **c == ' ').count();
    if lead > 3 || chars.get(lead) != Some(&'[') {
        return None;
    }
    let close =
        (lead..chars.len().saturating_sub(1)).find(|&i| chars[i] == ']' && chars[i + 1] == ':')?;
    let mut at = close + 2;
    while chars
        .get(at)
        .is_some_and(|c| c.is_whitespace() && *c != '\n')
    {
        at += 1;
    }
    let angled = chars.get(at) == Some(&'<');
    let start = at + usize::from(angled);
    let len = chars[start..]
        .iter()
        .take_while(|c| {
            if angled {
                **c != '>'
            } else {
                !c.is_whitespace()
            }
        })
        .count();
    (len > 0).then_some((start, len))
}
