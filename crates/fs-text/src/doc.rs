//! Document / block model (ADR 0034, 0042 §1).
//!
//! A Markdown or code file is a document with **one** block ([`BlockKind::Raw`]); a block
//! document has many. Both are the same model: text + marks.

use crate::id::{new_id, Uuid};
use crate::mark::{Mark, MarkSet};
use crate::text::Text;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    /// Whole file text (code mode / external file). Exported verbatim.
    Raw,
    Paragraph,
    /// Heading is a *block attribute*, not a mark.
    Heading { level: u8 },
    /// Fenced code; `info` is the info string (language).
    Code { info: String },
    /// Verbatim HTML block.
    Html,
    ThematicBreak,
    /// Cells separated by `'\t'`, rows by `'\n'` in the block text; first row is the header.
    Table { align: Vec<Align> },
    /// YAML front matter (without the `---` fences).
    FrontMatter,
}

/// Enclosing structure of a block. `inst` identifies one instance (so consecutive blocks
/// of the same list item / quote can be told apart from the next item).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Container {
    Quote { inst: u32 },
    /// `> [!type]± title`
    Callout { inst: u32, ty: String, title: String, fold: Option<char> },
    Item { inst: u32, list: u32, number: Option<u64>, marker: char, tight: bool, task: Option<bool> },
    Footnote { inst: u32, label: String },
}

impl Container {
    pub fn inst(&self) -> u32 {
        match self {
            Container::Quote { inst } | Container::Callout { inst, .. } | Container::Item { inst, .. } | Container::Footnote { inst, .. } => *inst,
        }
    }
    pub fn set_inst(&mut self, new: u32) {
        match self {
            Container::Quote { inst } | Container::Callout { inst, .. } | Container::Item { inst, .. } | Container::Footnote { inst, .. } => *inst = new,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockAttrs {
    pub kind: BlockKind,
    pub containers: Vec<Container>,
    /// Obsidian block id (`^id`), without the caret.
    pub block_ref: Option<String>,
    /// Heading attributes `{#id .class}` (verbatim, braces included).
    pub heading_attrs: Option<String>,
    pub extra: BTreeMap<String, String>,
}

impl BlockAttrs {
    pub fn new(kind: BlockKind) -> Self {
        Self { kind, containers: Vec::new(), block_ref: None, heading_attrs: None, extra: BTreeMap::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub id: Uuid,
    pub text: Text,
    pub marks: MarkSet,
    pub attrs: BlockAttrs,
}

impl Block {
    pub fn new(kind: BlockKind, text: &str) -> Self {
        Self { id: new_id(), text: Text::new(text), marks: MarkSet::new(), attrs: BlockAttrs::new(kind) }
    }
    pub fn paragraph(text: &str) -> Self {
        Self::new(BlockKind::Paragraph, text)
    }

    /// Insert text; marks move per their expand rules.
    pub fn insert(&mut self, at: usize, s: &str) {
        let at = at.min(self.text.len());
        let n = s.chars().count();
        self.text.insert(at, s);
        self.marks.on_insert(at, n);
    }
    /// Delete a char range; marks whose whole text is gone are removed and returned.
    pub fn delete(&mut self, start: usize, end: usize) -> Vec<Mark> {
        let end = end.min(self.text.len());
        let start = start.min(end);
        self.text.delete(start..end);
        self.marks.on_delete(start, end)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub blocks: Vec<Block>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    pub fn new() -> Self {
        Self { id: new_id(), blocks: Vec::new() }
    }
    /// A file (Markdown, code, …) as a one-block document.
    pub fn from_file_text(text: &str) -> Self {
        Self { id: new_id(), blocks: vec![Block::new(BlockKind::Raw, text)] }
    }
    pub fn block(&self, id: Uuid) -> Option<&Block> {
        self.blocks.iter().find(|b| b.id == id)
    }
    pub fn block_mut(&mut self, id: Uuid) -> Option<&mut Block> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }
    pub fn block_index(&self, id: Uuid) -> Option<usize> {
        self.blocks.iter().position(|b| b.id == id)
    }
    /// Text of a [`BlockKind::Raw`] one-block document.
    pub fn raw_text(&self) -> Option<String> {
        match self.blocks.as_slice() {
            [b] if b.attrs.kind == BlockKind::Raw => Some(b.text.to_string()),
            _ => None,
        }
    }
    /// Equality ignoring all ids (documents, blocks, marks); container instance numbers are
    /// compared by structure (relabelled in order of first appearance).
    pub fn same_content(&self, other: &Document) -> bool {
        fn norm(d: &Document) -> Vec<(String, BlockAttrs, Vec<Mark>)> {
            let mut map: std::collections::HashMap<(bool, u32), u32> = Default::default();
            let mut relabel = |list: bool, i: u32| {
                let n = map.len() as u32;
                *map.entry((list, i)).or_insert(n)
            };
            d.blocks
                .iter()
                .map(|b| {
                    let mut a = b.attrs.clone();
                    for c in &mut a.containers {
                        let i = relabel(false, c.inst());
                        c.set_inst(i);
                        if let Container::Item { list, .. } = c {
                            *list = relabel(true, *list);
                        }
                    }
                    let marks = b.marks.iter().cloned().map(|mut m| (m.id = Uuid::nil(), m).1).collect();
                    (b.text.to_string(), a, marks)
                })
                .collect()
        }
        norm(self) == norm(other)
    }
}
