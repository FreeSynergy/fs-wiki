//! Callouts: `> [!type]± title` as the first line of a quote.
//!
//! Pattern: Recogniser — inspects the first paragraph of a fresh quote and, on a match,
//! replaces the `Quote` container by a `Callout` and strips the header line from the text.

use super::state::Imp;
use crate::doc::{Block, Container};

impl Imp<'_> {
    /// `> [!type]± title` as first line of the first paragraph of a quote.
    pub(super) fn try_callout(&mut self, block: &mut Block, line: Option<usize>, raw: &str) {
        let text = block.text.to_string();
        let first = text.split('\n').next().unwrap_or("");
        let Some(rest) = first.strip_prefix("[!") else {
            return;
        };
        let Some(close) = rest.find(']') else { return };
        let ty = &rest[..close];
        if ty.is_empty()
            || !ty
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return;
        }
        let mut after = &rest[close + 1..];
        let mut fold = None;
        if let Some(c) = after.chars().next().filter(|c| *c == '+' || *c == '-') {
            fold = Some(c);
            after = &after[1..];
        }
        if !after.is_empty() && !after.starts_with(' ') {
            return;
        }
        // the title as written (`[x](a.md)`, escapes), not as it reads
        let title = raw
            .trim_start()
            .strip_prefix("[!")
            .and_then(|r| r.split_once(']'))
            .map(|(_, t)| t.strip_prefix(['+', '-']).unwrap_or(t).trim().to_string())
            .filter(|t| !t.is_empty() || after.trim().is_empty())
            .unwrap_or_else(|| after.trim().to_string());
        let first_len = first.chars().count();
        let carrier = first_len == block.text.len();
        block.delete(0, if carrier { first_len } else { first_len + 1 });
        let inst = self.inst();
        let callout = Container::Callout {
            inst,
            ty: ty.to_lowercase(),
            title,
            fold,
        };
        // replace the Quote container (last) by the callout
        let n = block.attrs.containers.len();
        block.attrs.containers[n - 1] = callout.clone();
        if line.is_some_and(|l| self.admon_lines.contains(&l)) {
            self.admon_insts.insert(inst);
        }
        self.containers[n - 1] = callout;
        if ty != ty.to_lowercase() {
            // `[!NOTE]` stays `[!NOTE]` when written again
            block
                .attrs
                .extra
                .insert("callout-type".into(), ty.to_string());
        }
        if carrier {
            block
                .attrs
                .extra
                .insert("callout-header".into(), "1".into());
        }
    }
}
