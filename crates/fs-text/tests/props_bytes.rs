//! Property tests over a wide alphabet (tab, `~`, `|`, `\`, `!`, `:`, brackets, braces, entities,
//! CR, `é`): untouched text comes back byte for byte, whatever it is; and whatever is done to the
//! model, the export either reads back as the model or says (`ExportReport`) what it could not
//! write exactly, while untouched blocks stay byte for byte.
//!
//! No source is skipped. What the canonical writer cannot say is listed in `explained` and the
//! tests check it: outside that list an export must be exact.

mod common;
use fs_text::*;
use proptest::prelude::*;
use std::collections::HashMap;

/// Markdown-ish soup: every char that starts some syntax, plus blanks and line breaks.
const SOUP: &str = "[a-z0-9 \t\n\r*_~|\\\\!:#>`.()\\[\\]{}<&;^+=\"/$é-]{0,160}";

fn export(d: &Document) -> (String, ExportReport) {
    export_markdown_report(d, ExportOptions::default())
}

/// The model without any kept source: only the canonical writer is under test.
fn plain(d: &Document) -> Document {
    let mut p = d.clone();
    for b in &mut p.blocks {
        b.attrs.source = None;
    }
    p
}

fn text_of(b: &Block) -> Vec<char> {
    b.text.chars().collect()
}

/// What the canonical writer cannot say exactly (see DESIGN.md, "Known limits"); an export may
/// report these blocks as inexact, anything else must be exact.
fn explained(d: &Document) -> bool {
    d.blocks.iter().any(|b| {
        let t = text_of(b);
        let emph = |m: &&Mark| {
            matches!(
                m.kind,
                MarkKind::Strong | MarkKind::Emphasis | MarkKind::Strike
            )
        };
        let marks: Vec<&Mark> = b.marks.iter().filter(emph).collect();
        // emphasis inside emphasis of the same kind: the cut may differ
        let nested = marks.iter().enumerate().any(|(i, a)| {
            marks[i + 1..]
                .iter()
                .any(|o| a.kind == o.kind && a.start < o.end && o.start < a.end)
        });
        // an inline HTML tag at the start of a line reads as an HTML block; one over several
        // lines inside a container takes the container's prefix
        let html = b.marks.iter().any(|m| {
            m.kind == MarkKind::Html
                && ((m.start > 0 && t[m.start - 1] == '\n')
                    || (!b.attrs.containers.is_empty()
                        && t[m.start..m.end.min(t.len())].contains(&'\n')))
        });
        // an HTML block or tag that never closes swallows what follows
        let open_html = (b.attrs.kind == BlockKind::Html
            || b.marks.iter().any(|m| m.kind == MarkKind::Html))
            && b.text.to_string().contains("<!")
            && !b.text.to_string().contains('>');
        // `* * *` is a rule; a loose list of empty items has no paragraph to show it
        let empties = (b.text.is_empty() && b.attrs.containers.len() >= 3)
            || (d.blocks.len() > 1 && b.text.is_empty() && !b.attrs.containers.is_empty());
        // a blank file is one empty paragraph that owns the blanks
        let blank = b.text.is_empty() && b.attrs.containers.is_empty();
        // flanking: `*(*x` cannot close when punctuation meets a letter
        let flanking = marks.iter().any(|m| {
            t.get(m.start).is_some_and(char::is_ascii_punctuation)
                || m.end > 0 && t.get(m.end - 1).is_some_and(char::is_ascii_punctuation)
        });
        nested || html || open_html || empties || blank || flanking
    })
}

proptest! {
    /// An untouched import-export gives the text back byte for byte, whatever it is.
    #[test]
    fn untouched_round_trip_is_byte_identical(src in SOUP) {
        let d = import_markdown(&src);
        prop_assert_eq!(export_markdown(&d, ExportOptions::default()), src);
    }

    /// The same with CRLF line ends and a BOM.
    #[test]
    fn crlf_and_bom_are_kept(src in SOUP) {
        let src = format!("\u{feff}{}", src.replace('\n', "\r\n"));
        let d = import_markdown(&src);
        prop_assert_eq!(export_markdown(&d, ExportOptions::default()), src);
    }

    /// The model survives JSON with its kept source.
    #[test]
    fn json_keeps_the_source(src in SOUP) {
        let d = import_markdown(&src);
        let back: Document = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        prop_assert_eq!(export_markdown(&back, ExportOptions::default()), src);
    }

    /// The canonical writer alone: the text reads back as the model, or the report says which
    /// blocks it could not write exactly; outside the known limits there are none.
    #[test]
    fn canonical_form_reads_back_or_says_so(src in SOUP) {
        let d = plain(&import_markdown(&src));
        let (out, report) = export(&d);
        let exact = common::reads_back_as(&d, &out);
        if report.inexact.is_empty() {
            prop_assert!(exact, "{:?} -> {:?}: the report is empty but the text reads differently", src, out);
        }
        if !explained(&d) {
            prop_assert!(report.inexact.is_empty() && exact, "{:?} -> {:?}: {:?}", src, out, report);
        }
    }

    /// Whatever is done to the model: the export reads back as the model or the report says
    /// where not, and an untouched block (same content, same predecessor) stays byte for byte
    /// unless the report names it as rewritten because its kept text would read differently.
    #[test]
    fn edits_keep_untouched_blocks_and_say_what_is_inexact(
        src in SOUP, kind in 0u8..7, at in 0usize..8, word in "[a-z*#>\\- \\n]{0,6}"
    ) {
        let mut d = import_markdown(&src);
        if d.blocks.is_empty() {
            return Ok(());
        }
        let before: HashMap<_, _> = d
            .blocks
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id, (b.fingerprint(), i.checked_sub(1).map(|p| d.blocks[p].id))))
            .collect();
        let i = at % d.blocks.len();
        match kind {
            0 => { let n = d.blocks[i].text.len(); d.blocks[i].splice(0, n.min(1), &word); }
            1 => { d.blocks.remove(i); }
            2 => d.blocks[i].attrs.containers.clear(),
            3 => d.blocks[i].attrs.kind = BlockKind::Paragraph,
            4 => d.blocks.insert(i, Block::paragraph(&word)),
            5 => { let j = (at / 2) % d.blocks.len(); d.blocks.swap(i, j); }
            _ => { let b = d.blocks[i].clone(); d.blocks.push(b); }
        }
        if d.blocks.is_empty() {
            return Ok(());
        }
        let (out, report) = export(&d);
        if report.inexact.is_empty() {
            prop_assert!(common::reads_back_as(&d, &out), "{:?} edit {} at {} {:?} -> {:?}", src, kind, at, word, out);
        }
        // a word changed inside a paragraph or heading (no definition, no HTML) cannot change
        // what its neighbours mean: nothing may be rewritten
        let plain_text = |b: &Block| {
            matches!(b.attrs.kind, BlockKind::Paragraph | BlockKind::Heading { .. })
                && !b.text.to_string().contains("]:")
                && !b.text.to_string().contains('<')
                && !b.text.to_string().contains('\n')
                && !b.text.is_empty()
        };
        if kind == 0
            && plain_text(&d.blocks[i])
            && d.blocks.iter().all(|b| b.attrs.containers.is_empty())
            && !src.contains("]:") && !src.contains('<') {
            prop_assert!(report.rewritten.is_empty(), "{:?} edit 0 at {} {:?}: rewrote neighbours {:?}", src, at, word, report);
        }
        // text that stands only in a kept source (next to the block, in no block) is in the output
        // or named in the report: the report cannot lie about it
        for b in &d.blocks {
            let Some(s) = &b.attrs.source else { continue };
            let lead = s.text.get(..s.own).unwrap_or("");
            let tail = if s.tail && s.end > 0 { s.text.get(s.end..).unwrap_or("") } else { "" };
            for line in lead.lines().chain(tail.lines()) {
                let t = line.trim_matches(|c: char| c.is_whitespace() || c == '>');
                if t.is_empty() || t == "<!-- -->" {
                    continue;
                }
                prop_assert!(
                    out.contains(line.trim()) || report.inexact.iter().any(|i| i.why.contains(line.trim())),
                    "{:?} edit {} at {} {:?}: {:?} is only in a kept source, not in {:?} nor the report {:?}",
                    src, kind, at, word, line, out, report
                );
            }
        }
        // untouched blocks: their kept text is in the output
        for (n, b) in d.blocks.iter().enumerate() {
            let Some(s) = &b.attrs.source else { continue };
            let prev = n.checked_sub(1).map(|p| d.blocks[p].id);
            let group_untouched = std::iter::once(b)
                .chain(d.blocks[n + 1..].iter().take(s.rest.len()))
                .all(|x| before.get(&x.id).is_some_and(|(fp, _)| *fp == x.fingerprint()));
            let same_place = before.get(&b.id).is_some_and(|(_, p)| *p == prev);
            if group_untouched && same_place && s.prev == prev && !report.rewritten.contains(&b.id) {
                prop_assert!(
                    out.contains(&s.text),
                    "{:?} edit {} at {} {:?}: untouched block lost its text {:?} in {:?} ({:?})",
                    src, kind, at, word, s.text, out, report
                );
            }
        }
    }
}
