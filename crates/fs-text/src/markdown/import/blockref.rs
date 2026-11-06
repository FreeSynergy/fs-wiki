//! Obsidian block references (`text ^id`).
//!
//! Pattern: Extractor — a trailing ` ^block-id` is moved from the text to a block attribute
//! (only plain text with a letter in the id: `mc ^2` and `` `^1` `` are text).

use crate::doc::Block;

/// Trailing ` ^block-id` becomes a block attribute.
pub(super) fn extract_block_ref(b: &mut Block) {
    let t = b.text.to_string();
    if let Some(pos) = t.rfind(" ^") {
        let id = &t[pos + 2..];
        // an id has a letter (`^2` after `mc` is an exponent) and is plain text, not code or a link
        let start = t[..pos].chars().count();
        let end = b.text.len();
        let plain = !b.marks.iter().any(|m| m.end > start && m.start < end);
        if plain
            && id.chars().any(|c| c.is_ascii_alphabetic())
            && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            b.delete(start, end);
            b.attrs.block_ref = Some(id.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_the_ref_off() {
        let mut b = Block::paragraph("text ^blk-1");
        extract_block_ref(&mut b);
        assert_eq!(
            (b.text.to_string(), b.attrs.block_ref.as_deref()),
            ("text".into(), Some("blk-1"))
        );
        let mut b = Block::paragraph("a ^not valid");
        extract_block_ref(&mut b);
        assert_eq!(b.attrs.block_ref, None);
    }
}
