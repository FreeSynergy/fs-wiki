//! Markdown -> model (ADR 0042: text + marks).
//!
//! CommonMark + GFM (tables, task lists, strikethrough), footnotes, YAML front matter,
//! wiki links, block refs `^id`, callouts `> [!note]` and mkdocs `!!! note "Title"`.

use crate::doc::*;
use crate::id::new_id;
use crate::mark::*;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, LinkType, MetadataBlockKind, Options, Parser, Tag, TagEnd};
use std::collections::HashSet;

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_WIKILINKS
}

/// Hard line break is stored as U+2028 inside block text.
pub const HARD_BREAK: char = '\u{2028}';

/// Convert mkdocs admonitions (`!!! note "Title"` + indented body) into callout syntax.
pub fn convert_admonitions(src: &str) -> String {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    let mut fence: Option<(char, usize)> = None;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim_start();
        let ind = line.len() - t.len();
        // fenced code tracking (only the plain top-level / indented fences)
        if let Some(c) = t.chars().next().filter(|c| *c == '`' || *c == '~') {
            let n = t.chars().take_while(|x| *x == c).count();
            if n >= 3 {
                match fence {
                    None => fence = Some((c, n)),
                    Some((fc, fnn)) if fc == c && n >= fnn && t[n..].trim().is_empty() => fence = None,
                    _ => {}
                }
            }
        }
        if fence.is_none() && ind < 4 {
            if let Some((fold, ty, title)) = parse_admonition_header(t) {
                let mut hdr = format!("{}> [!{}]{}", &line[..ind], ty, fold.map(|c| c.to_string()).unwrap_or_default());
                if !title.is_empty() {
                    hdr.push(' ');
                    hdr.push_str(&title);
                }
                out.push(hdr);
                i += 1;
                let base = &line[..ind];
                // body: indented >= ind+4 or blank (blank only if followed by more body)
                let mut body: Vec<&str> = Vec::new();
                while i < lines.len() {
                    let l = lines[i];
                    if l.trim().is_empty() {
                        // lookahead
                        let mut j = i;
                        while j < lines.len() && lines[j].trim().is_empty() {
                            j += 1;
                        }
                        if j < lines.len() && indent_of(lines[j]) >= ind + 4 {
                            body.extend(&lines[i..j]);
                            i = j;
                            continue;
                        }
                        break;
                    }
                    if indent_of(l) >= ind + 4 {
                        body.push(l);
                        i += 1;
                    } else {
                        break;
                    }
                }
                for l in body {
                    if l.trim().is_empty() {
                        out.push(format!("{}>", base));
                    } else {
                        out.push(format!("{}> {}", base, &l[ind + 4..]));
                    }
                }
                continue;
            }
        }
        out.push(line.to_string());
        i += 1;
    }
    out.join("\n")
}

fn indent_of(l: &str) -> usize {
    l.len() - l.trim_start_matches(' ').len()
}

fn parse_admonition_header(t: &str) -> Option<(Option<char>, String, String)> {
    let (rest, fold) = if let Some(r) = t.strip_prefix("!!!") {
        (r, None)
    } else if let Some(r) = t.strip_prefix("???+") {
        (r, Some('+'))
    } else if let Some(r) = t.strip_prefix("???") {
        (r, Some('-'))
    } else {
        return None;
    };
    if !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.trim();
    let ty: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
    if ty.is_empty() {
        return None;
    }
    let after = rest[ty.len()..].trim();
    let title = if after.is_empty() {
        String::new()
    } else if after.starts_with('"') && after.ends_with('"') && after.len() >= 2 {
        after[1..after.len() - 1].to_string()
    } else {
        return None;
    };
    Some((fold, ty.to_lowercase(), title))
}

struct Leaf {
    kind: BlockKind,
    text: String,
    len: usize,
    marks: Vec<Mark>,
    open: Vec<(MarkKind, usize, MarkValue, usize)>,
    seq: usize,
    order: Vec<usize>,
    heading_attrs: Option<String>,
    start: Option<usize>,
    glue: bool,
    dashes: Option<String>,
}

impl Leaf {
    fn new(kind: BlockKind) -> Self {
        Self { kind, text: String::new(), len: 0, marks: Vec::new(), open: Vec::new(), seq: 0, order: Vec::new(), heading_attrs: None, start: None, glue: false, dashes: None }
    }
    fn push(&mut self, s: &str) {
        self.text.push_str(s);
        self.len += s.chars().count();
    }
}

struct Imp {
    blocks: Vec<Block>,
    containers: Vec<Container>,
    fresh_quote: Vec<bool>,
    leaf: Option<Leaf>,
    next_inst: u32,
    lists: Vec<(u32, Option<u64>, u64, bool)>, // inst, start, next index, tight
    pending_tight: bool,
    item_start: Vec<usize>,
}

/// Import Markdown text as a block document.
pub fn import_markdown(src: &str) -> Document {
    let src = convert_admonitions(&src.replace("\r\n", "\n"));
    let events: Vec<(Event, std::ops::Range<usize>)> = Parser::new_ext(&src, options()).into_offset_iter().collect();
    // Which lists are loose? A paragraph directly inside an item makes its list loose.
    let mut loose: HashSet<usize> = HashSet::new();
    {
        let mut stack: Vec<(bool, usize)> = Vec::new(); // (is_item, list_event_idx)
        let mut list_stack: Vec<usize> = Vec::new();
        for (i, (e, _)) in events.iter().enumerate() {
            match e {
                Event::Start(Tag::List(_)) => {
                    list_stack.push(i);
                    stack.push((false, i));
                }
                Event::Start(Tag::Item) => stack.push((true, *list_stack.last().unwrap_or(&0))),
                Event::Start(Tag::Paragraph) => {
                    if let Some((true, l)) = stack.last() {
                        loose.insert(*l);
                    }
                    stack.push((false, 0));
                }
                Event::Start(_) => stack.push((false, 0)),
                Event::End(t) => {
                    if matches!(t, TagEnd::List(_)) {
                        list_stack.pop();
                    }
                    stack.pop();
                }
                _ => {}
            }
        }
    }
    let mut s = Imp { blocks: Vec::new(), containers: Vec::new(), fresh_quote: Vec::new(), leaf: None, next_inst: 1, lists: Vec::new(), pending_tight: true, item_start: Vec::new() };
    let mut in_code = false;
    let mut link_stack: Vec<bool> = Vec::new();
    for (idx, (ev, range)) in events.iter().enumerate() {
        match ev {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    s.flush();
                    s.leaf = Some(Leaf::new(BlockKind::Paragraph));
                }
                Tag::Heading { level, id, classes, attrs } => {
                    s.flush();
                    let mut l = Leaf::new(BlockKind::Heading { level: heading_level(*level) });
                    if id.is_some() || !classes.is_empty() || !attrs.is_empty() {
                        let mut parts = Vec::new();
                        if let Some(id) = id {
                            parts.push(format!("#{id}"));
                        }
                        for c in classes {
                            parts.push(format!(".{c}"));
                        }
                        for (k, v) in attrs {
                            parts.push(match v {
                                Some(v) => format!("{k}={v}"),
                                None => k.to_string(),
                            });
                        }
                        l.heading_attrs = Some(format!("{{{}}}", parts.join(" ")));
                    }
                    s.leaf = Some(l);
                }
                Tag::CodeBlock(k) => {
                    s.flush();
                    let info = match k {
                        CodeBlockKind::Fenced(i) => i.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    s.leaf = Some(Leaf::new(BlockKind::Code { info }));
                    in_code = true;
                }
                Tag::HtmlBlock => {
                    s.flush();
                    s.leaf = Some(Leaf::new(BlockKind::Html));
                }
                Tag::BlockQuote(_) => {
                    s.flush();
                    let inst = s.inst();
                    s.containers.push(Container::Quote { inst });
                    s.fresh_quote.push(true);
                }
                Tag::List(start) => {
                    s.flush();
                    let inst = s.inst();
                    s.lists.push((inst, *start, 0, !loose.contains(&idx)));
                }
                Tag::Item => {
                    s.flush();
                    let inst = s.inst();
                    let (list, start, n, tight) = {
                        let l = s.lists.last_mut().expect("item in list");
                        let r = (l.0, l.1, l.2, l.3);
                        l.2 += 1;
                        r
                    };
                    let src_slice = src[range.start..].trim_start_matches([' ', '\t']);
                    let marker = if start.is_some() {
                        src_slice.trim_start_matches(|c: char| c.is_ascii_digit()).chars().next().unwrap_or('.')
                    } else {
                        src_slice.chars().next().unwrap_or('-')
                    };
                    s.containers.push(Container::Item { inst, list, number: start.map(|st| st + n), marker, tight, task: None });
                    s.item_start.push(s.blocks.len());
                }
                Tag::FootnoteDefinition(label) => {
                    s.flush();
                    let inst = s.inst();
                    s.containers.push(Container::Footnote { inst, label: label.to_string() });
                }
                Tag::Table(al) => {
                    s.flush();
                    let align = al
                        .iter()
                        .map(|a| match a {
                            pulldown_cmark::Alignment::None => Align::None,
                            pulldown_cmark::Alignment::Left => Align::Left,
                            pulldown_cmark::Alignment::Center => Align::Center,
                            pulldown_cmark::Alignment::Right => Align::Right,
                        })
                        .collect();
                    s.leaf = Some(Leaf::new(BlockKind::Table { align }));
                    s.pending_tight = true; // "no row yet"
                }
                Tag::TableHead | Tag::TableRow => {
                    let l = s.leaf.as_mut().unwrap();
                    if !s.pending_tight {
                        l.push("\n");
                    }
                    s.pending_tight = false;
                    // first cell of the row needs no tab
                    s.lists.push((u32::MAX, None, 0, true));
                }
                Tag::TableCell => {
                    let first = s.lists.last().map(|l| l.2 == 0).unwrap_or(true);
                    if let Some(l) = s.lists.last_mut() {
                        l.2 += 1;
                    }
                    if !first {
                        s.leaf.as_mut().unwrap().push("\t");
                    }
                }
                Tag::MetadataBlock(k) => {
                    s.flush();
                    let mut l = Leaf::new(BlockKind::FrontMatter);
                    if matches!(k, MetadataBlockKind::PlusesStyle) {
                        l.heading_attrs = Some("+++".into());
                    }
                    s.leaf = Some(l);
                }
                Tag::Emphasis => s.open(MarkKind::Emphasis, under(&src, range.start)),
                Tag::Strong => s.open(MarkKind::Strong, under(&src, range.start)),
                Tag::Strikethrough => s.open(MarkKind::Strike, MarkValue::None),
                Tag::Link { link_type, dest_url, title, .. } | Tag::Image { link_type, dest_url, title, .. } => {
                    let is_image = matches!(tag, Tag::Image { .. });
                    s.ensure_leaf();
                    let style = match link_type {
                        LinkType::Autolink | LinkType::Email => LinkStyle::Autolink,
                        LinkType::WikiLink { .. } if is_image => LinkStyle::Embed,
                        LinkType::WikiLink { .. } => LinkStyle::Wiki,
                        _ => LinkStyle::Inline,
                    };
                    let mut lv = LinkValue::new(dest_url.to_string(), style);
                    if !title.is_empty() {
                        lv.title = Some(title.to_string());
                    }
                    link_stack.push(is_image);
                    s.open(if is_image { MarkKind::Image } else { MarkKind::Link }, MarkValue::Link(lv));
                }
                _ => {}
            },
            Event::End(end) => match end {
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::HtmlBlock | TagEnd::MetadataBlock(_) => s.flush(),
                TagEnd::CodeBlock => {
                    in_code = false;
                    s.flush();
                }
                TagEnd::BlockQuote(_) => {
                    s.flush();
                    s.containers.pop();
                    s.fresh_quote.pop();
                }
                TagEnd::List(_) => {
                    s.flush();
                    s.lists.pop();
                }
                TagEnd::Item | TagEnd::FootnoteDefinition => {
                    s.flush();
                    if matches!(end, TagEnd::Item) && s.item_start.pop() == Some(s.blocks.len()) {
                        // empty list item: keep it as an empty paragraph
                        s.leaf = Some(Leaf::new(BlockKind::Paragraph));
                        s.flush();
                    }
                    s.containers.pop();
                }
                TagEnd::Table => {
                    s.flush();
                }
                TagEnd::TableHead | TagEnd::TableRow => {
                    s.lists.pop();
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => s.close(),
                TagEnd::Link | TagEnd::Image => {
                    link_stack.pop();
                    s.close();
                }
                _ => {}
            },
            Event::Text(t) => {
                s.ensure_leaf();
                s.leaf.as_mut().unwrap().push(t);
            }
            Event::Code(c) => {
                s.ensure_leaf();
                let l = s.leaf.as_mut().unwrap();
                let st = l.len;
                l.push(c);
                let end = l.len;
                l.seq += 1;
                l.order.push(l.seq);
                l.marks.push(Mark::new(MarkKind::Code, st, end, MarkValue::None));
            }
            Event::InlineMath(t) | Event::DisplayMath(t) => {
                s.ensure_leaf();
                s.leaf.as_mut().unwrap().push(t);
            }
            Event::Html(h) | Event::InlineHtml(h) => {
                s.ensure_leaf();
                let l = s.leaf.as_mut().unwrap();
                let st = l.len;
                l.push(h);
                if l.kind != BlockKind::Html {
                    let end = l.len;
                    l.seq += 1;
                    l.order.push(l.seq);
                    l.marks.push(Mark::new(MarkKind::Html, st, end, MarkValue::None));
                }
            }
            Event::FootnoteReference(label) => {
                s.ensure_leaf();
                let l = s.leaf.as_mut().unwrap();
                l.seq += 1;
                l.order.push(l.seq);
                l.marks.push(Mark::point(MarkKind::FootnoteRef, l.len, MarkValue::Name(label.to_string())));
            }
            Event::SoftBreak => {
                s.ensure_leaf();
                s.leaf.as_mut().unwrap().push("\n");
            }
            Event::HardBreak => {
                s.ensure_leaf();
                let mut b = [0u8; 4];
                s.leaf.as_mut().unwrap().push(HARD_BREAK.encode_utf8(&mut b));
            }
            Event::Rule => {
                s.flush();
                let mut l = Leaf::new(BlockKind::ThematicBreak);
                l.glue = glue_before(&src, range.start);
                s.leaf = Some(l);
                s.flush();
            }
            Event::TaskListMarker(done) => {
                if let Some(Container::Item { task, .. }) = s.containers.last_mut() {
                    *task = Some(*done);
                }
            }
        }
        if let Some(l) = s.leaf.as_mut() {
            if l.start.is_none() {
                l.start = Some(range.start);
                l.glue = glue_before(&src, range.start);
                if let BlockKind::Table { .. } = l.kind {
                    l.dashes = table_dashes(&src[range.clone()]);
                }
            }
        }
    }
    let _ = in_code;
    s.flush();
    Document { id: new_id(), blocks: s.blocks }
}

fn heading_level(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

impl Imp {
    fn inst(&mut self) -> u32 {
        let i = self.next_inst;
        self.next_inst += 1;
        i
    }
    fn ensure_leaf(&mut self) {
        if self.leaf.is_none() {
            self.leaf = Some(Leaf::new(BlockKind::Paragraph));
        }
    }
    fn open(&mut self, k: MarkKind, v: MarkValue) {
        self.ensure_leaf();
        let l = self.leaf.as_mut().unwrap();
        let at = l.len;
        l.seq += 1;
        let seq = l.seq;
        l.open.push((k, at, v, seq));
    }
    fn close(&mut self) {
        if let Some(l) = self.leaf.as_mut() {
            if let Some((k, st, v, seq)) = l.open.pop() {
                let end = l.len;
                l.marks.push(Mark::new(k, st, end, v));
                l.order.push(seq);
            }
        }
    }

    fn flush(&mut self) {
        let Some(mut leaf) = self.leaf.take() else { return };
        if matches!(leaf.kind, BlockKind::Code { .. } | BlockKind::Html | BlockKind::FrontMatter) && leaf.text.ends_with('\n') {
            leaf.text.pop();
            leaf.len -= 1;
        }
        let mut attrs = BlockAttrs::new(leaf.kind.clone());
        attrs.containers = self.containers.clone();
        match (&leaf.kind, leaf.heading_attrs.take()) {
            (BlockKind::Heading { .. }, a) => attrs.heading_attrs = a,
            (BlockKind::FrontMatter, Some(f)) => {
                attrs.extra.insert("fence".into(), f);
            }
            _ => {}
        }
        if leaf.glue {
            attrs.extra.insert("glue".into(), "1".into());
        }
        if let Some(d) = leaf.dashes.take() {
            attrs.extra.insert("dashes".into(), d);
        }
        let mut block = Block { id: new_id(), text: crate::text::Text::new(&leaf.text), marks: MarkSet::from_vec(sorted_by_open(leaf.marks, &leaf.order)), attrs };
        let was_fresh = self.fresh_quote.last().copied().unwrap_or(false);
        if let Some(f) = self.fresh_quote.last_mut() {
            *f = false;
        }
        if matches!(block.attrs.kind, BlockKind::Paragraph | BlockKind::Heading { .. }) {
            extract_block_ref(&mut block);
        }
        if was_fresh && block.attrs.kind == BlockKind::Paragraph {
            self.try_callout(&mut block);
        }
        self.blocks.push(block);
    }

    /// `> [!type]± title` as first line of the first paragraph of a quote.
    fn try_callout(&mut self, block: &mut Block) {
        let text = block.text.to_string();
        let first = text.split('\n').next().unwrap_or("");
        let Some(rest) = first.strip_prefix("[!") else { return };
        let Some(close) = rest.find(']') else { return };
        let ty = &rest[..close];
        if ty.is_empty() || !ty.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
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
        let title = after.trim().to_string();
        let first_len = first.chars().count();
        let total = block.text.len();
        let carrier = first_len == total;
        block.delete(0, if carrier { first_len } else { first_len + 1 });
        let inst = self.inst();
        let callout = Container::Callout { inst, ty: ty.to_lowercase(), title, fold };
        // replace the Quote container (last) by the callout
        let n = block.attrs.containers.len();
        block.attrs.containers[n - 1] = callout.clone();
        self.containers[n - 1] = callout;
        if carrier {
            block.attrs.extra.insert("callout-header".into(), "1".into());
        }
    }
}

/// Trailing ` ^block-id` (Obsidian) becomes a block attribute.
fn extract_block_ref(b: &mut Block) {
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

/// Was the line before `off` non-blank (block glued to its predecessor)?
fn glue_before(src: &str, off: usize) -> bool {
    let ls = src[..off].rfind('\n').map(|i| i + 1).unwrap_or(0);
    if ls == 0 {
        return false;
    }
    let prev = src[..ls - 1].rsplit('\n').next().unwrap_or("");
    !prev.trim_start_matches([' ', '>', '\t']).trim().is_empty()
}

/// Widths of the delimiter row segments (`|---|:--:|`), to write tables back as they were.
fn table_dashes(table_src: &str) -> Option<String> {
    let line = table_src.lines().nth(1)?;
    let t = line.trim().trim_start_matches('>').trim();
    let t = t.strip_prefix('|')?.strip_suffix('|')?;
    let spaced = if t.starts_with(' ') { "s" } else { "" };
    Some(format!("{spaced}{}", t.split('|').map(|c| c.trim().chars().count().to_string()).collect::<Vec<_>>().join(",")))
}

/// Outer marks (opened first) come first among marks with equal ranges.
fn sorted_by_open(marks: Vec<Mark>, order: &[usize]) -> Vec<Mark> {
    let mut v: Vec<(usize, Mark)> = order.iter().copied().zip(marks).collect();
    v.sort_by_key(|(o, _)| *o);
    v.into_iter().map(|(_, m)| m).collect()
}

/// Remember underscore delimiters (`_em_`, `__strong__`) so export can write them back.
fn under(src: &str, at: usize) -> MarkValue {
    if src[at..].starts_with('_') {
        MarkValue::Name("_".into())
    } else {
        MarkValue::None
    }
}
