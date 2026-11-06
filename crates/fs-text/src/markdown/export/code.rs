//! Fenced code blocks.
//!
//! Pattern: Template — the fence is one backtick longer than the longest run inside.

/// Lines of a fenced code block with the given info string.
pub(super) fn render_code(content: &str, info: &str) -> Vec<String> {
    let mut longest = 0;
    for l in content.split('\n') {
        let n = l.trim_start().chars().take_while(|c| *c == '`').count();
        longest = longest.max(n);
    }
    let fence = "`".repeat((longest + 1).max(3));
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
    }
}
