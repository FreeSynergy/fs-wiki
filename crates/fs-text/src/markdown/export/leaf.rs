//! Rendering one block to lines.
//!
//! Pattern: Strategy by block kind — a `match` on [`BlockKind`] picks the renderer
//! (`code`, `table`, `inline` for text).

use super::code::render_code;
use super::inline::{choose, inline, Ctx, Variant};
use super::options::ExportOptions;
use super::table::render_table;
use crate::doc::{Block, BlockKind};
use crate::mark::Mark;

fn lines(s: &str) -> Vec<String> {
    s.split('\n').map(String::from).collect()
}

pub(super) fn render_block(b: &Block, opts: ExportOptions) -> Vec<String> {
    let chars: Vec<char> = b.text.chars().collect();
    let marks: Vec<&Mark> = b.marks.iter().collect();
    let text = |heading: bool, v: Variant| {
        inline(
            &chars,
            &marks,
            (0, chars.len()),
            Ctx {
                table: false,
                heading,
                opts,
            },
            v,
        )
    };
    match &b.attrs.kind {
        BlockKind::Raw | BlockKind::Html => lines(&b.text.to_string()),
        BlockKind::Paragraph => choose(b, opts, &|v| {
            let mut s = text(false, v);
            if let Some(r) = &b.attrs.block_ref {
                s.push_str(" ^");
                s.push_str(r);
            }
            lines(&s)
        }),
        BlockKind::Heading { level } => choose(b, opts, &|v| {
            let mut s = format!("{} ", "#".repeat(*level as usize));
            s.push_str(&text(true, v));
            if let Some(a) = &b.attrs.heading_attrs {
                s.push(' ');
                s.push_str(a);
            }
            if let Some(r) = &b.attrs.block_ref {
                s.push_str(" ^");
                s.push_str(r);
            }
            vec![s.trim_end().to_string()]
        }),
        BlockKind::Code { info } => render_code(&b.text.to_string(), info),
        BlockKind::ThematicBreak => vec!["---".into()],
        BlockKind::FrontMatter => {
            let f = b.attrs.extra.get("fence").map_or("---", String::as_str);
            let mut v = vec![f.to_string()];
            if !b.text.is_empty() {
                v.extend(lines(&b.text.to_string()));
            }
            v.push(f.to_string());
            v
        }
        BlockKind::Table { align } => {
            choose(b, opts, &|v| render_table(b, &chars, &marks, align, opts, v))
        }
    }
}
