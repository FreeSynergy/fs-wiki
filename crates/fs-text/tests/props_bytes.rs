//! Property tests over a wide alphabet (tab, `~`, `|`, `\`, `!`, `:`, brackets, braces, entities).

mod common;
use fs_text::*;
use proptest::prelude::*;

/// Markdown-ish soup: every char that starts some syntax, plus blanks and line breaks.
const SOUP: &str = "[a-z0-9 \t\n\r*_~|\\\\!:#>`.()\\[\\]{}<&;^+=\"/$é-]{0,160}";

/// Which chars each emphasis kind covers (nested or touching spans of one kind count once),
/// then the document without those marks: everything else must be `same_content` up to layout style.
fn coverage_and_rest(d: &Document) -> (Vec<Vec<(MarkKind, usize)>>, Document) {
    let mut rest = d.clone();
    let mut cover = Vec::new();
    for b in &mut rest.blocks {
        let mut c = Vec::new();
        let ids: Vec<_> = b
            .marks
            .iter()
            .filter(|m| {
                matches!(
                    m.kind,
                    MarkKind::Strong | MarkKind::Emphasis | MarkKind::Strike
                )
            })
            .map(|m| (m.id, m.kind, m.start, m.end))
            .collect();
        for (id, k, s, e) in ids {
            c.extend((s..e).map(|i| (k, i)));
            b.marks.remove(id);
        }
        c.sort_unstable();
        c.dedup();
        cover.push(c);
    }
    (cover, rest)
}

/// Known limits (see DESIGN.md) that are not part of the canonical property: emphasis inside
/// emphasis of the same kind cannot always be written back with the same cut, and inline HTML
/// over several lines inside a container takes the container's prefix as part of its text; three
/// nested empty bullets are written `* * *`.
fn known_limit(d: &Document) -> bool {
    let html_lines = d.blocks.iter().any(|b| {
        !b.attrs.containers.is_empty()
            && b.marks.iter().any(|m| {
                m.kind == MarkKind::Html
                    && b.text
                        .to_string()
                        .chars()
                        .skip(m.start)
                        .take(m.end - m.start)
                        .any(|c| c == '\n')
            })
    });
    // `* * *` (three nested empty bullets) is a thematic break whatever is done
    let empty_nest = d
        .blocks
        .iter()
        .any(|b| b.text.is_empty() && b.attrs.containers.len() >= 3)
        // a loose list of empty items has no paragraph to show that it is loose
        || (d.blocks.len() > 1
            && d.blocks.iter().any(|b| b.text.is_empty() && !b.attrs.containers.is_empty()));
    // an inline HTML tag at the start of a line may read as an HTML block
    let html_at_line_start = d.blocks.iter().any(|b| {
        let t: Vec<char> = b.text.chars().collect();
        b.marks
            .iter()
            .any(|m| m.kind == MarkKind::Html && m.start > 0 && t[m.start - 1] == '\n')
    });
    html_lines
        || html_at_line_start
        || empty_nest
        || d.blocks.iter().any(|b| {
            let v: Vec<&Mark> = b
                .marks
                .iter()
                .filter(|m| {
                    matches!(
                        m.kind,
                        MarkKind::Strong | MarkKind::Emphasis | MarkKind::Strike
                    )
                })
                .collect();
            v.iter().enumerate().any(|(i, a)| {
                v[i + 1..]
                    .iter()
                    .any(|o| a.kind == o.kind && a.start < o.end && o.start < a.end)
            })
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

    /// The canonical writer alone reads back as the same content (emphasis spans may be re-cut,
    /// what they cover stays).
    #[test]
    fn canonical_form_reads_back(src in SOUP) {
        let d1 = import_markdown(&src);
        if src.contains("<!") || src.contains("]:") || known_limit(&d1) {
            return Ok(());
        }
        let out = common::canonical(&d1);
        let d2 = import_markdown(&out);
        let ((c1, r1), (c2, r2)) = (coverage_and_rest(&d1), coverage_and_rest(&d2));
        prop_assert!(
            common::without_style(&r1).same_content(&common::without_style(&r2)) && c1 == c2,
            "{:?} -> {:?}", src, out);
    }

    /// The model survives JSON with its kept source.
    #[test]
    fn json_keeps_the_source(src in SOUP) {
        let d = import_markdown(&src);
        let back: Document = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        prop_assert_eq!(export_markdown(&back, ExportOptions::default()), src);
    }

    /// Whatever is done to the model, the file written with kept sources reads back as the model
    /// whenever the canonical writer alone can do so (the exporter proves its output).
    #[test]
    fn edited_export_reads_back(src in SOUP, kind in 0u8..7, at in 0usize..8, word in "[a-z*#>\\- \\n]{0,6}") {
        let mut d = import_markdown(&src);
        if d.blocks.is_empty() {
            return Ok(());
        }
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
        if src.contains("<!") || src.contains("]:") || d.blocks.is_empty() || known_limit(&d) {
            return Ok(());
        }
        let canon = common::canonical(&d);
        if !common::reads_back_as(&d, &canon) {
            return Ok(());
        }
        let out = export_markdown(&d, ExportOptions::default());
        prop_assert!(common::reads_back_as(&d, &out), "{:?} edit {} at {} {:?} -> {:?}", src, kind, at, word, out);
    }
}
