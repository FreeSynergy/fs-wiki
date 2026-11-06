//! Obsidian block references (`text ^id`).
//!
//! Pattern: Extractor — a trailing ` ^block-id` is moved from the text to a block attribute.

use crate::doc::Block;

/// Trailing ` ^block-id` becomes a block attribute.
pub(super) fn extract_block_ref(b: &mut Block) {
    let t = b.text.to_string();
    if let Some(pos) = t.rfind(" ^") {
        let id = &t[pos + 2..];
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            let start = t[..pos].chars().count();
            let end = b.text.len();
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
