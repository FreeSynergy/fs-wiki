//! Fenced code blocks.
//!
//! Pattern: Template — the fence is one backtick longer than the longest run inside.

/// Lines of a fenced code block with the given info string.
///
/// Pattern: Template — the fence is one char longer than the longest run inside.
pub(super) fn render_code(content: &str, info: &str) -> Vec<String> {
    // a backtick in the info string cannot stand in a backtick fence: use tildes
    let ch = if info.contains('`') { '~' } else { '`' };
    let mut longest = 0;
    for l in content.split('\n') {
        let n = l.trim_start().chars().take_while(|c| *c == ch).count();
        longest = longest.max(n);
    }
    let fence = ch.to_string().repeat((longest + 1).max(3));
    let mut v = vec![format!("{fence}{info}")];
    v.extend(content.split('\n').map(String::from));
    v.push(fence);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_outgrows_content() {
        assert_eq!(render_code("a", "rs"), ["```rs", "a", "```"]);
        assert_eq!(render_code("```x", ""), ["````", "```x", "````"]);
        assert_eq!(render_code("a", "x`y"), ["~~~x`y", "a", "~~~"]);
    }
}
