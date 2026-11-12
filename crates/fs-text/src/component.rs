//! Component blocks: a living component of the program (task board, map, calendar, form, people, files ...) inside a page.
//!
//! Pattern: Value object + Recogniser — a component block is `{ component (typed, versioned id), configuration, data source
//! (a reference through the settings mechanism, ADR 0066), static fallback }`. In Markdown it is written as
//!
//! ```text
//! <!-- fs-component {"component":"fs.tasks.board","version":1,"config":{...},"source":{"setting":"...","params":{}}} -->
//! the static fallback: ordinary Markdown (text, a table, an image)
//! <!-- /fs-component -->
//! ```
//!
//! **Why comments, not a fenced block.** An HTML comment is invisible in every Markdown renderer (GitHub, mkdocs, a plain
//! viewer), and the fallback between the two comments is ordinary Markdown that they render as it is: the page reads
//! complete everywhere, the component only adds life where the program draws it. A fenced block with its own info string
//! would show its configuration as code to every reader that does not know it, so the fallback would not be what they see.
//! A comment is also something the existing importer already keeps verbatim (an `Html` block), so nothing about the
//! round trip changes: the two markers and the blocks between are ordinary blocks and come back byte for byte, with the
//! kept source text of untouched blocks and from the canonical writer for changed ones. No page of the documentation
//! contains the marker, so none of them reads differently.
//!
//! The configuration is JSON on one line; a `--` inside would end the comment, so the writer writes it `--` (valid JSON,
//! read back as `--`).
//!
//! Rules for the program that draws it (written in `DESIGN.md`): a component block shows only what the viewer may see; on a
//! public site only data that is released publicly; on the web it lives through Blinc-Web (Wasm), otherwise the fallback is shown.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::doc::{BlockKind, Document};

/// What opens a component block.
pub const OPEN: &str = "<!-- fs-component ";
/// What closes it.
pub const CLOSE: &str = "<!-- /fs-component -->";

/// Where the component gets its data: a reference through the settings mechanism, never the data itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataSource {
    /// The setting that names the source (`tasks.board.project`).
    pub setting: String,
    /// What the setting is asked with (a filter, an id ...); free for the component.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub params: Value,
}

/// A component block without its fallback (the blocks between the markers).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentSpec {
    /// The type of the component (`fs.tasks.board`).
    pub component: String,
    /// Its version: a component of another version may read the configuration differently.
    pub version: u32,
    /// The configuration, as the component defines it.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub config: Value,
    /// Where it gets its data (none: it needs none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<DataSource>,
}

/// A component block found in a document: its spec and where its blocks are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentRef {
    /// The spec read from the opening marker.
    pub spec: ComponentSpec,
    /// Index of the block that holds the opening marker.
    pub open: usize,
    /// Index of the block that holds the closing marker.
    pub close: usize,
}

impl ComponentRef {
    /// The indexes of the fallback blocks (between the two markers).
    pub fn fallback(&self) -> std::ops::Range<usize> {
        self.open + 1..self.close
    }
}

impl ComponentSpec {
    /// The opening marker, on one line.
    ///
    /// # Panics
    /// Never: a spec is always valid JSON.
    pub fn open_marker(&self) -> String {
        let json = serde_json::to_string(self).expect("a spec is JSON");
        // `--` would end the comment: it can only stand inside a string, where `-` is the same text.
        let json = json.replace("--", "-\\u002d");
        format!("{OPEN}{json} -->")
    }

    /// The Markdown of a component block: the markers around `fallback` (ordinary Markdown).
    pub fn markdown(&self, fallback: &str) -> String {
        format!(
            "{}\n\n{}\n\n{CLOSE}\n",
            self.open_marker(),
            fallback.trim_matches('\n')
        )
    }

    /// The spec in an opening marker (the text of an `Html` block); `None` when it is not a component marker, `Some(Err)`
    /// when it is one that does not read (it then stays an ordinary comment and the caller can say why).
    pub fn from_marker(html: &str) -> Option<Result<Self, String>> {
        let body = html.trim().strip_prefix(OPEN.trim_end())?;
        let body = body.strip_suffix("-->")?;
        if !body.starts_with(' ') {
            return None;
        }
        Some(
            serde_json::from_str(body.trim())
                .map_err(|e| format!("a component block does not read: {e}")),
        )
    }
}

/// The component blocks of `doc`, in order. A marker without its closing one, or one that does not read, is not a component
/// block (it stays the comment it is); `problems` says which, with the index of its block.
pub fn find_components(doc: &Document) -> (Vec<ComponentRef>, Vec<(usize, String)>) {
    let (mut found, mut problems) = (Vec::new(), Vec::new());
    let html = |i: usize| {
        let b = &doc.blocks[i];
        (b.attrs.kind == BlockKind::Html).then(|| b.text.to_string())
    };
    let mut i = 0;
    while i < doc.blocks.len() {
        let Some(spec) = html(i).and_then(|t| ComponentSpec::from_marker(&t)) else {
            i += 1;
            continue;
        };
        let close = (i + 1..doc.blocks.len()).find(|&j| html(j).is_some_and(|t| t.trim() == CLOSE));
        match (spec, close) {
            (Ok(spec), Some(close)) => {
                found.push(ComponentRef {
                    spec,
                    open: i,
                    close,
                });
                i = close + 1;
            }
            (Ok(_), None) => {
                problems.push((i, "a component block is not closed".to_owned()));
                i += 1;
            }
            (Err(why), _) => {
                problems.push((i, why));
                i += 1;
            }
        }
    }
    (found, problems)
}
