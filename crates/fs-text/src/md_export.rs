//! Model -> Markdown. Canonical style: `-` bullets are kept as imported, `*emphasis*`,
//! `**strong**`, fenced code, padded-header tables. Layers (names, variables,
//! explanations) are *not* written unless [`ExportOptions::layers`] is set (ADR 0042 §2).

use crate::doc::*;
use crate::mark::*;
use crate::md_import::HARD_BREAK;

#[derive(Clone, Copy, Debug, Default)]
pub struct ExportOptions {
    /// Also write layer marks with attribute syntax (`{{name}}`, `[text]{#name}`).
    pub layers: bool,
}

/// Export a document. A one-block [`BlockKind::Raw`] document exports its text verbatim.
pub fn export_markdown(doc: &Document, opts: ExportOptions) -> String {
    export_blocks(&doc.blocks, opts)
}

pub fn export_blocks(blocks: &[Block], opts: ExportOptions) -> String {
    let mut out: Vec<String> = Vec::new();
    if let [b] = blocks {
        if b.attrs.kind == BlockKind::Raw {
            return b.text.to_string();
        }
    }
    let mut prev: &[Container] = &[];
    for (bi, b) in blocks.iter().enumerate() {
        let cs = &b.attrs.containers[..];
        if bi > 0 {
            let k = common(prev, cs);
            let tight = tight_join(prev, cs, k);
            let glued = b.attrs.extra.contains_key("glue") && !(blocks[bi - 1].attrs.kind == BlockKind::Paragraph && b.attrs.kind == BlockKind::Paragraph && common(prev, cs) == cs.len() && prev.len() == cs.len());
            if !(tight || glued) {
                let pfx: String = cs[..k].iter().map(cont_prefix).collect();
                out.push(pfx.trim_end().to_string());
            }
        }
        render_in_containers(b, prev, &mut out, opts);
        prev = cs;
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

/// No blank line between two blocks of the same tight list (or inside one tight item).
fn tight_join(a: &[Container], b: &[Container], k: usize) -> bool {
    if k == b.len() {
        return matches!(b.last(), Some(Container::Item { tight: true, .. }));
    }
    let Container::Item { tight: true, list, .. } = &b[k] else { return false };
    match a.get(k) {
        Some(Container::Item { list: al, .. }) => al == list,
        Some(_) => false,
        None => k > 0 && matches!(b[k - 1], Container::Item { .. }),
    }
}

fn common(a: &[Container], b: &[Container]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x.inst() == y.inst()).count()
}

fn marker(c: &Container) -> String {
    match c {
        Container::Quote { .. } | Container::Callout { .. } => "> ".into(),
        Container::Item { number, marker, task, .. } => {
            let mut s = match number {
                Some(n) => format!("{n}{marker} "),
                None => format!("{marker} "),
            };
            match task {
                Some(true) => s.push_str("[x] "),
                Some(false) => s.push_str("[ ] "),
                None => {}
            }
            s
        }
        Container::Footnote { label, .. } => format!("[^{label}]: "),
    }
}
fn cont_prefix(c: &Container) -> String {
    match c {
        Container::Quote { .. } | Container::Callout { .. } => "> ".into(),
        Container::Item { number, .. } => " ".repeat(number.map(|n| n.to_string().len() + 2).unwrap_or(2)),
        Container::Footnote { .. } => "    ".into(),
    }
}

fn render_in_containers(b: &Block, prev: &[Container], out: &mut Vec<String>, opts: ExportOptions) {
    let cs = &b.attrs.containers;
    let k = common(prev, cs);
    let mut fresh: Vec<bool> = (0..cs.len()).map(|d| d >= k).collect();
    let prefix = |fresh: &[bool], upto: usize| -> String { cs[..upto].iter().enumerate().map(|(d, c)| if fresh[d] { marker(c) } else { cont_prefix(c) }).collect() };
    // callout headers for new callouts
    for d in k..cs.len() {
        if let Container::Callout { ty, title, fold, .. } = &cs[d] {
            let mut h = prefix(&fresh, d);
            h.push_str("> [!");
            h.push_str(ty);
            h.push(']');
            if let Some(f) = fold {
                h.push(*f);
            }
            if !title.is_empty() {
                h.push(' ');
                h.push_str(title);
            }
            out.push(h.trim_end().to_string());
            for f in fresh.iter_mut().take(d + 1) {
                *f = false;
            }
        }
    }
    if b.attrs.extra.contains_key("callout-header") && b.text.is_empty() {
        return;
    }
    let lines = render_block(b, opts);
    for line in lines {
        let mut l = prefix(&fresh, cs.len());
        l.push_str(&line);
        out.push(l.trim_end().to_string());
        fresh.iter_mut().for_each(|f| *f = false);
    }
}

fn render_block(b: &Block, opts: ExportOptions) -> Vec<String> {
    let chars: Vec<char> = b.text.chars().collect();
    let marks: Vec<&Mark> = b.marks.iter().collect();
    match &b.attrs.kind {
        BlockKind::Raw => b.text.to_string().split('\n').map(String::from).collect(),
        BlockKind::Paragraph => {
            let mut s = inline(&chars, &marks, 0, chars.len(), Ctx { table: false, heading: false, opts });
            if let Some(r) = &b.attrs.block_ref {
                s.push_str(" ^");
                s.push_str(r);
            }
            s.split('\n').map(String::from).collect()
        }
        BlockKind::Heading { level } => {
            let mut s = format!("{} ", "#".repeat(*level as usize));
            s.push_str(&inline(&chars, &marks, 0, chars.len(), Ctx { table: false, heading: true, opts }));
            if let Some(a) = &b.attrs.heading_attrs {
                s.push(' ');
                s.push_str(a);
            }
            if let Some(r) = &b.attrs.block_ref {
                s.push_str(" ^");
                s.push_str(r);
            }
            vec![s.trim_end().to_string()]
        }
        BlockKind::Code { info } => {
            let content = b.text.to_string();
            let mut longest = 0;
            for l in content.split('\n') {
                let t = l.trim_start();
                let n = t.chars().take_while(|c| *c == '`').count();
                longest = longest.max(n);
            }
            let fence = "`".repeat((longest + 1).max(3));
            let mut v = vec![format!("{fence}{info}")];
            v.extend(content.split('\n').map(String::from));
            v.push(fence);
            v
        }
        BlockKind::Html => b.text.to_string().split('\n').map(String::from).collect(),
        BlockKind::ThematicBreak => vec!["---".into()],
        BlockKind::FrontMatter => {
            let f = b.attrs.extra.get("fence").map(String::as_str).unwrap_or("---");
            let mut v = vec![f.to_string()];
            if !b.text.is_empty() {
                v.extend(b.text.to_string().split('\n').map(String::from));
            }
            v.push(f.to_string());
            v
        }
        BlockKind::Table { align } => render_table(b, &chars, &marks, align, opts),
    }
}

fn render_table(b: &Block, chars: &[char], marks: &[&Mark], align: &[Align], opts: ExportOptions) -> Vec<String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut pos = 0;
    // split into rows and cells by '\n' and '\t'
    let mut row = Vec::new();
    let mut cell_start = 0;
    for i in 0..=chars.len() {
        let sep = if i == chars.len() { Some('\n') } else { Some(chars[i]).filter(|c| *c == '\n' || *c == '\t') };
        if let Some(c) = sep {
            row.push(inline(chars, marks, cell_start, i, Ctx { table: true, heading: false, opts }));
            cell_start = i + 1;
            if c == '\n' {
                rows.push(std::mem::take(&mut row));
            }
        }
        pos = i;
    }
    let _ = pos;
    let mut out = Vec::new();
    let ncols = align.len().max(rows.first().map(|r| r.len()).unwrap_or(0));
    let fmt_row = |r: &Vec<String>| -> String {
        let mut s = String::from("|");
        for c in 0..ncols {
            s.push(' ');
            s.push_str(r.get(c).map(String::as_str).unwrap_or(""));
            s.push_str(" |");
        }
        s
    };
    if let Some(h) = rows.first() {
        out.push(fmt_row(h));
        let mut d = String::from("|");
        for c in 0..ncols {
            let given: Option<usize> = b.attrs.extra.get("dashes").and_then(|d| d.split(',').nth(c)).and_then(|x| x.parse().ok());
            let w = given.unwrap_or_else(|| h.get(c).map(|x| x.chars().count()).unwrap_or(0) + 2).max(3);
            let a = align.get(c).copied().unwrap_or(Align::None);
            let cell = match a {
                Align::None => "-".repeat(w),
                Align::Left => format!(":{}", "-".repeat(w - 1)),
                Align::Right => format!("{}:", "-".repeat(w - 1)),
                Align::Center => format!(":{}:", "-".repeat(w - 2)),
            };
            d.push_str(&cell);
            d.push('|');
        }
        out.push(d);
    }
    for r in rows.iter().skip(1) {
        out.push(fmt_row(r));
    }
    out
}

#[derive(Clone, Copy)]
struct Ctx {
    table: bool,
    heading: bool,
    opts: ExportOptions,
}

fn is_atomic(m: &Mark) -> bool {
    match m.kind {
        MarkKind::Code | MarkKind::Html => true,
        MarkKind::Link | MarkKind::Image => matches!(m.link().map(|l| l.style), Some(LinkStyle::Wiki | LinkStyle::Embed | LinkStyle::Autolink)),
        _ => false,
    }
}

fn exported(m: &Mark, o: ExportOptions) -> bool {
    match m.kind {
        MarkKind::Variable => o.opts_layers(),
        _ => m.kind.is_markdown_native() || (o.layers && m.kind == MarkKind::Passage),
    }
}
impl ExportOptions {
    fn opts_layers(self) -> bool {
        self.layers
    }
}

struct Em {
    s: usize,
    e: usize,
    m: usize, // index into marks
    atomic: bool,
}

fn raw_slice(chars: &[char], s: usize, e: usize) -> String {
    chars[s..e].iter().collect()
}

fn inline(chars: &[char], marks: &[&Mark], lo: usize, hi: usize, cx: Ctx) -> String {
    // clip to window
    let mut ems: Vec<Em> = Vec::new();
    for (i, m) in marks.iter().enumerate() {
        if !exported(m, cx.opts) {
            continue;
        }
        if m.start > hi || m.end < lo || (m.start == hi && !m.is_point()) {
            continue;
        }
        if m.is_point() {
            if m.start < lo || m.start > hi {
                continue;
            }
        } else if m.end <= lo || m.start >= hi {
            continue;
        }
        let s = m.start.max(lo) - lo;
        let e = m.end.min(hi) - lo;
        ems.push(Em { s, e, m: i, atomic: is_atomic(m) });
    }
    let chars = &chars[lo..hi];
    let n = chars.len();
    // atomic ranges win; drop overlapping atomics, snap other boundaries
    ems.sort_by_key(|e| (e.s, std::cmp::Reverse(e.e)));
    let mut atoms: Vec<(usize, usize)> = Vec::new();
    ems.retain(|e| {
        if !e.atomic {
            return true;
        }
        if atoms.iter().any(|a| e.s < a.1 && a.0 < e.e) {
            return false;
        }
        atoms.push((e.s, e.e));
        true
    });
    for e in ems.iter_mut().filter(|e| !e.atomic) {
        for a in &atoms {
            if a.0 < e.s && e.s < a.1 {
                e.s = a.0;
            }
            if a.0 < e.e && e.e < a.1 {
                e.e = a.1;
            }
        }
    }
    // non-point ranges that became empty are dropped
    ems.retain(|e| e.atomic || e.e > e.s || marks[e.m].is_point());

    let mut out = String::new();
    let mut stack: Vec<usize> = Vec::new(); // indices into ems
    let mut opened = vec![false; ems.len()];
    let mut p = 0usize;
    loop {
        // closes
        loop {
            let Some(j) = stack.iter().position(|&i| ems[i].e <= p) else { break };
            let above: Vec<usize> = stack.split_off(j);
            // close from the top
            for &i in above.iter().rev() {
                out.push_str(&close_str(marks[ems[i].m], cx));
            }
            for &i in above.iter().skip(1) {
                if ems[i].e > p {
                    out.push_str(&open_str(marks[ems[i].m], cx));
                    stack.push(i);
                } else {
                    // also ends here; already closed
                }
            }
        }
        // points
        for (k, e) in ems.iter().enumerate() {
            if e.s == p && marks[e.m].is_point() && !opened[k] {
                opened[k] = true;
                out.push_str(&point_str(marks[e.m]));
            }
        }
        // opens, longest first
        let mut atom_end = None;
        let mut cands: Vec<usize> = (0..ems.len()).filter(|&k| !opened[k] && ems[k].s == p && !marks[ems[k].m].is_point()).collect();
        cands.sort_by_key(|&k| (!ems[k].atomic, std::cmp::Reverse(ems[k].e)));
        for k in cands {
            opened[k] = true;
            let e = &ems[k];
            let m = marks[e.m];
            if e.atomic {
                if atom_end.is_none() {
                    let mut raw = raw_slice(chars, e.s, e.e);
                    if cx.table {
                        raw = raw.replace('|', "\\|");
                    }
                    out.push_str(&atomic_str(m, &raw));
                    atom_end = Some(e.e);
                }
            } else {
                out.push_str(&open_str(m, cx));
                stack.push(k);
            }
        }
        if let Some(end) = atom_end {
            p = end;
            continue;
        }
        if p >= n {
            break;
        }
        let in_link = stack.iter().any(|&i| matches!(marks[ems[i].m].kind, MarkKind::Link | MarkKind::Image));
        escape_into(&mut out, chars, p, cx, in_link);
        p += 1;
    }
    // anything left open (shouldn't be): close
    while let Some(i) = stack.pop() {
        out.push_str(&close_str(marks[ems[i].m], cx));
    }
    if cx.heading && out.ends_with('#') {
        let l = out.len() - 1;
        out.insert(l, '\\');
    }
    out
}

fn open_str(m: &Mark, _cx: Ctx) -> String {
    match m.kind.spec().markdown {
        MdSyntax::Delim(o, _) => o.to_string(),
        MdSyntax::Link => "[".into(),
        MdSyntax::Image => "![".into(),
        MdSyntax::Layer => "[".into(),
        _ => String::new(),
    }
}

fn close_str(m: &Mark, _cx: Ctx) -> String {
    match m.kind.spec().markdown {
        MdSyntax::Delim(_, c) => c.to_string(),
        MdSyntax::Link | MdSyntax::Image => match m.link() {
            Some(l) => format!("]({}{})", dest(&l.path), l.title.as_ref().map(|t| format!(" \"{}\"", t.replace('"', "\\\""))).unwrap_or_default()),
            None => "]()".into(),
        },
        MdSyntax::Layer => match &m.value {
            MarkValue::Name(n) => format!("]{{#{n}}}"),
            _ => "]".into(),
        },
        _ => String::new(),
    }
}

fn dest(d: &str) -> String {
    let opens = d.matches('(').count();
    let closes = d.matches(')').count();
    if d.is_empty() || d.contains(char::is_whitespace) || opens != closes {
        format!("<{d}>")
    } else {
        d.to_string()
    }
}

fn point_str(m: &Mark) -> String {
    match &m.value {
        MarkValue::Name(l) if m.kind == MarkKind::FootnoteRef => format!("[^{l}]"),
        MarkValue::Variable { name, .. } => format!("{{{{{name}}}}}"),
        _ => String::new(),
    }
}

fn atomic_str(m: &Mark, content: &str) -> String {
    match m.kind {
        MarkKind::Code => {
            let mut longest = 0;
            let mut run = 0;
            for c in content.chars() {
                if c == '`' {
                    run += 1;
                    longest = longest.max(run);
                } else {
                    run = 0;
                }
            }
            let f = "`".repeat(longest + 1);
            let pad = content.starts_with('`') || content.ends_with('`') || (content.starts_with(' ') && content.ends_with(' ') && content.trim() != "");
            let content = content.replace('\n', " ");
            if pad {
                format!("{f} {content} {f}")
            } else {
                format!("{f}{content}{f}")
            }
        }
        MarkKind::Html => content.to_string(),
        _ => {
            let l = m.link().expect("link value");
            match l.style {
                LinkStyle::Autolink => format!("<{content}>"),
                LinkStyle::Wiki | LinkStyle::Embed => {
                    let bang = if l.style == LinkStyle::Embed { "!" } else { "" };
                    if content == l.path {
                        format!("{bang}[[{}]]", l.path)
                    } else {
                        format!("{bang}[[{}|{}]]", l.path, content)
                    }
                }
                LinkStyle::Inline => unreachable!(),
            }
        }
    }
}

fn escape_into(out: &mut String, chars: &[char], p: usize, cx: Ctx, brackets: bool) {
    let c = chars[p];
    let prev = if p > 0 { Some(chars[p - 1]) } else { None };
    let next = chars.get(p + 1).copied();
    let ws = |o: Option<char>| o.map(|c| c.is_whitespace()).unwrap_or(true);
    let line_start = !cx.table && !cx.heading && (p == 0 || matches!(chars[p - 1], '\n' | HARD_BREAK));
    let next_blank = matches!(next, None | Some(' ') | Some('\n') | Some(HARD_BREAK));
    let esc = match c {
        HARD_BREAK => {
            out.push_str("\\\n");
            return;
        }
        '\t' => {
            out.push(' ');
            return;
        }
        '\\' => next.map(|n| n.is_ascii_punctuation() || n == '\n').unwrap_or(true),
        '`' => true,
        '*' => !(ws(prev) && ws(next)),
        '_' => !((ws(prev) && ws(next)) || (prev.map(char::is_alphanumeric).unwrap_or(false) && next.map(char::is_alphanumeric).unwrap_or(false))),
        '[' => brackets || next == Some('^') || next == Some('[') || (line_start && matches!(next, Some(' ' | 'x' | 'X' | '!'))),
        ']' => brackets || next == Some('('),
        '<' => matches!(next, Some(n) if n.is_alphanumeric() || n == '/' || n == '!' || n == '?'),
        '&' => {
            let rest: String = chars[p + 1..].iter().take(12).collect();
            rest.starts_with('#') || rest.split(';').next().map(|w| rest.contains(';') && !w.is_empty() && w.chars().all(|c| c.is_ascii_alphanumeric())).unwrap_or(false)
        }
        '~' => prev == Some('~') || next == Some('~'),
        '|' => cx.table,
        '#' if line_start => {
            let n = chars[p..].iter().take_while(|c| **c == '#').count();
            n <= 6 && matches!(chars.get(p + n).copied(), None | Some(' ') | Some('\n') | Some(HARD_BREAK))
        }
        '>' if line_start => true,
        '+' | '-' if line_start => {
            next_blank || chars[p..].iter().take_while(|c| **c != '\n').all(|c| *c == '-' || *c == ' ')
        }
        '=' if line_start => chars[p..].iter().take_while(|c| **c != '\n').all(|c| *c == '='),
        '.' | ')' if !cx.table && !cx.heading => {
            // ordered list marker at line start: digits then . or ) then blank
            let mut i = p;
            while i > 0 && chars[i - 1].is_ascii_digit() {
                i -= 1;
            }
            let digits = p - i;
            let at_start = i == 0 || matches!(chars[i - 1], '\n' | HARD_BREAK);
            digits > 0 && digits <= 9 && at_start && next_blank
        }
        _ => false,
    };
    if esc {
        out.push('\\');
    }
    out.push(c);
}
