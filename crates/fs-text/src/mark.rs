//! Marks: one mechanism for formatting, links, names, variables and explanations
//! (ADR 0033). A mark is stand-off: it lives next to the text, never inside it.
//!
//! * `start == end`  -> **point**: something is *inserted* here (variable, footnote ref).
//! * `start <  end`  -> **range**: the text is *named* / *replaced* / formatted.

use crate::id::{new_id, Uuid};
use serde::{Deserialize, Serialize};

/// What happens when text is typed exactly at a mark boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expand {
    /// Text typed at the start joins the mark.
    Before,
    /// Text typed at the end joins the mark.
    After,
    Both,
    None,
}
impl Expand {
    pub fn before(self) -> bool {
        matches!(self, Expand::Before | Expand::Both)
    }
    pub fn after(self) -> bool {
        matches!(self, Expand::After | Expand::Both)
    }
}

/// How two marks of the same kind and value behave when they overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Overlap {
    /// Union into one mark (bold + bold).
    Merge,
    /// Both coexist, each keeps its identity (two links, two names).
    Multiple,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MarkKind {
    Strong,
    Emphasis,
    Code,
    Strike,
    /// Link; value = [`LinkValue`]. Target is a UUID plus the readable path.
    Link,
    /// Image / embed; value = [`LinkValue`] (alt text is the marked range).
    Image,
    /// Verbatim inline HTML.
    Html,
    /// Named passage (range, value = name). ADR 0034 / 0033: content that can be reused.
    Passage,
    /// Variable placeholder (point). ADR 0033: filled in on instantiation.
    Variable,
    /// Own explanation replacing / muting the original text (range).
    Explanation,
    /// Footnote reference (point, value = label).
    FootnoteRef,
}

/// How a kind is written in Markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MdSyntax {
    /// `open text close`
    Delim(&'static str, &'static str),
    /// `[text](dest "title")`, `[[path|text]]`, `<url>`
    Link,
    /// `![alt](src)`
    Image,
    /// Verbatim.
    Raw,
    /// `[^label]`
    FootnoteRef,
    /// Only with `layers` export: attribute syntax (`{{name}}`, `[text]{#name}`).
    Layer,
}

/// Registry entry for a mark kind.
#[derive(Clone, Copy, Debug)]
pub struct KindSpec {
    pub kind: MarkKind,
    pub name: &'static str,
    pub expand: Expand,
    pub overlap: Overlap,
    /// May be a point (start == end)?
    pub point: bool,
    pub markdown: MdSyntax,
}

const fn spec(kind: MarkKind, name: &'static str, expand: Expand, overlap: Overlap, point: bool, markdown: MdSyntax) -> KindSpec {
    KindSpec { kind, name, expand, overlap, point, markdown }
}

/// The kind registry. (`heading` is a block attribute, see [`crate::BlockKind::Heading`].)
pub const REGISTRY: &[KindSpec] = &[
    spec(MarkKind::Strong, "strong", Expand::After, Overlap::Merge, false, MdSyntax::Delim("**", "**")),
    spec(MarkKind::Emphasis, "emphasis", Expand::After, Overlap::Merge, false, MdSyntax::Delim("*", "*")),
    spec(MarkKind::Code, "code", Expand::After, Overlap::Merge, false, MdSyntax::Delim("`", "`")),
    spec(MarkKind::Strike, "strike", Expand::After, Overlap::Merge, false, MdSyntax::Delim("~~", "~~")),
    spec(MarkKind::Link, "link", Expand::None, Overlap::Multiple, false, MdSyntax::Link),
    spec(MarkKind::Image, "image", Expand::None, Overlap::Multiple, false, MdSyntax::Image),
    spec(MarkKind::Html, "html", Expand::None, Overlap::Multiple, false, MdSyntax::Raw),
    spec(MarkKind::Passage, "passage", Expand::None, Overlap::Multiple, false, MdSyntax::Layer),
    spec(MarkKind::Variable, "variable", Expand::None, Overlap::Multiple, true, MdSyntax::Layer),
    spec(MarkKind::Explanation, "explanation", Expand::None, Overlap::Multiple, false, MdSyntax::Layer),
    spec(MarkKind::FootnoteRef, "footnote-ref", Expand::None, Overlap::Multiple, true, MdSyntax::FootnoteRef),
];

impl MarkKind {
    pub fn spec(self) -> &'static KindSpec {
        REGISTRY.iter().find(|s| s.kind == self).expect("every kind is registered")
    }
    /// Is this kind part of plain Markdown (so a file export contains it)?
    pub fn is_markdown_native(self) -> bool {
        !matches!(self.spec().markdown, MdSyntax::Layer)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkStyle {
    /// `[text](dest)`
    Inline,
    /// `<https://…>`
    Autolink,
    /// `[[path]]` / `[[path|label]]`
    Wiki,
    /// `![[path]]`
    Embed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkValue {
    /// Resolved target (page / block) once known; `None` for external or unresolved links.
    pub target: Option<Uuid>,
    /// Readable path or URL, exactly as written in the text layer.
    pub path: String,
    pub title: Option<String>,
    pub style: LinkStyle,
}

impl LinkValue {
    pub fn new(path: impl Into<String>, style: LinkStyle) -> Self {
        Self { target: None, path: path.into(), title: None, style }
    }
    /// `(page, fragment)` split at the first `#`.
    pub fn split_fragment(&self) -> (&str, Option<&str>) {
        match self.path.split_once('#') {
            Some((p, f)) => (p, Some(f)),
            None => (&self.path, None),
        }
    }
    /// Does the path point to an internal page (not an URL / mail)?
    pub fn is_internal(&self) -> bool {
        matches!(self.style, LinkStyle::Wiki | LinkStyle::Embed) || (!self.path.contains("://") && !self.path.starts_with("mailto:") && !self.path.starts_with('#'))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkValue {
    None,
    Link(LinkValue),
    /// Passage name, footnote label.
    Name(String),
    Variable { name: String, ty: Option<String> },
    Explanation { replacement: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    pub id: Uuid,
    pub kind: MarkKind,
    pub value: MarkValue,
    /// Char offsets into the block text. `start == end` = point.
    pub start: usize,
    pub end: usize,
    pub expand: Expand,
}

impl Mark {
    pub fn new(kind: MarkKind, start: usize, end: usize, value: MarkValue) -> Self {
        Self { id: new_id(), kind, value, start: start.min(end), end, expand: kind.spec().expand }
    }
    pub fn point(kind: MarkKind, at: usize, value: MarkValue) -> Self {
        Self::new(kind, at, at, value)
    }
    pub fn is_point(&self) -> bool {
        self.start == self.end
    }
    pub fn link(&self) -> Option<&LinkValue> {
        match &self.value {
            MarkValue::Link(l) => Some(l),
            _ => None,
        }
    }

    /// Move through an insertion of `len` chars at `at`.
    pub fn on_insert(&mut self, at: usize, len: usize) {
        if self.is_point() {
            // A point stays put (typed text lands after it) iff expand contains `before`.
            if self.start > at || (self.start == at && !self.expand.before()) {
                self.start += len;
            }
            self.end = self.start;
            return;
        }
        if self.start > at || (self.start == at && !self.expand.before()) {
            self.start += len;
        }
        if self.end > at || (self.end == at && self.expand.after()) {
            self.end += len;
        }
    }

    /// Move through a deletion of `a..b`. Returns `false` if the mark died: its whole text
    /// (or the point's surrounding text) is gone and it takes the mark along (ADR 0042 §6).
    pub fn on_delete(&mut self, a: usize, b: usize) -> bool {
        if a >= b {
            return true;
        }
        if self.is_point() {
            if self.start > a && self.start < b {
                return false;
            }
        } else if self.start >= a && self.end <= b {
            return false;
        }
        let m = |o: usize| if o <= a { o } else if o >= b { o - (b - a) } else { a };
        self.start = m(self.start);
        self.end = m(self.end);
        true
    }
}

/// The marks of one block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkSet {
    marks: Vec<Mark>,
}

impl MarkSet {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_vec(mut marks: Vec<Mark>) -> Self {
        marks.sort_by_key(|m| (m.start, std::cmp::Reverse(m.end)));
        Self { marks }
    }
    pub fn iter(&self) -> std::slice::Iter<'_, Mark> {
        self.marks.iter()
    }
    pub fn as_slice(&self) -> &[Mark] {
        &self.marks
    }
    pub fn len(&self) -> usize {
        self.marks.len()
    }
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }
    pub fn get(&self, id: Uuid) -> Option<&Mark> {
        self.marks.iter().find(|m| m.id == id)
    }
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut Mark> {
        self.marks.iter_mut().find(|m| m.id == id)
    }
    pub fn remove(&mut self, id: Uuid) -> Option<Mark> {
        let i = self.marks.iter().position(|m| m.id == id)?;
        Some(self.marks.remove(i))
    }
    /// Marks covering char `offset` (ranges: `start <= offset < end`; points: `== offset`).
    pub fn at(&self, offset: usize) -> impl Iterator<Item = &Mark> {
        self.marks.iter().filter(move |m| (m.start <= offset && offset < m.end) || (m.is_point() && m.start == offset))
    }

    /// Add a mark. Kinds with [`Overlap::Merge`] are merged with overlapping or touching
    /// marks of the same kind and value (the earliest id survives).
    pub fn add(&mut self, mut mark: Mark) -> Uuid {
        if mark.kind.spec().overlap == Overlap::Merge && !mark.is_point() {
            let mut i = 0;
            while i < self.marks.len() {
                let o = &self.marks[i];
                if o.kind == mark.kind && o.value == mark.value && o.start <= mark.end && mark.start <= o.end && !o.is_point() {
                    let o = self.marks.remove(i);
                    mark.start = mark.start.min(o.start);
                    mark.end = mark.end.max(o.end);
                    mark.id = o.id;
                } else {
                    i += 1;
                }
            }
        }
        let id = mark.id;
        let pos = self.marks.partition_point(|m| (m.start, std::cmp::Reverse(m.end)) <= (mark.start, std::cmp::Reverse(mark.end)));
        self.marks.insert(pos, mark);
        id
    }

    /// Remove `kind` from `start..end`: marks are cut or split around the range.
    pub fn unmark(&mut self, kind: MarkKind, start: usize, end: usize) {
        let mut out = Vec::new();
        for m in self.marks.drain(..) {
            if m.kind != kind || m.is_point() || m.end <= start || m.start >= end {
                out.push(m);
                continue;
            }
            if m.start < start {
                let mut l = m.clone();
                l.end = start;
                out.push(l);
            }
            if m.end > end {
                let mut r = m.clone();
                if m.start < start {
                    r.id = new_id();
                }
                r.start = end;
                out.push(r);
            }
        }
        *self = MarkSet::from_vec(out);
    }

    pub fn on_insert(&mut self, at: usize, len: usize) {
        for m in &mut self.marks {
            m.on_insert(at, len);
        }
        self.resort();
    }

    /// Returns the marks that died with the deleted text.
    pub fn on_delete(&mut self, a: usize, b: usize) -> Vec<Mark> {
        let mut dead = Vec::new();
        let mut keep = Vec::with_capacity(self.marks.len());
        for mut m in self.marks.drain(..) {
            let orig = m.clone();
            if m.on_delete(a, b) {
                keep.push(m);
            } else {
                dead.push(orig);
            }
        }
        self.marks = keep;
        self.resort();
        dead
    }

    fn resort(&mut self) {
        self.marks.sort_by_key(|m| (m.start, std::cmp::Reverse(m.end)));
    }

    /// Drop marks beyond `len` and clamp ends (defensive; used after bulk imports).
    pub fn clamp(&mut self, len: usize) {
        for m in &mut self.marks {
            m.start = m.start.min(len);
            m.end = m.end.min(len);
        }
    }

    /// Merge overlapping / touching marks of [`Overlap::Merge`] kinds.
    pub fn normalize(&mut self) {
        let old = std::mem::take(&mut self.marks);
        for m in old {
            self.add(m);
        }
    }
}
