//! Records of the Admin frontend made from the values of the core.
//!
//! Pattern: Adapter — `TextEntry`, `Node`, `RevisionInfo`, `TrashItem`, `LinkEntry` to `Record` (id and values by field
//! name of the description). Pure functions; the shape of every record is the description of `fs-wiki-core`.

use fs_admin_frontend::{Outcome, Record, Values};
use fs_objects::ObjectId;
use fs_wiki_core::{
    LinkEntry, Node, NodeKind, RevisionInfo, SearchHit, TextEntry, TextFormat, TrashItem,
};

use super::ctx::text;

fn record(id: String, fields: &[(&str, Vec<String>)]) -> Record {
    let mut values = Values::new();
    values.insert("id".to_owned(), vec![id.clone()]);
    for (name, v) in fields {
        if !v.is_empty() {
            values.insert((*name).to_owned(), v.clone());
        }
    }
    Record { id, values }
}

fn kind(template: bool) -> Vec<String> {
    vec![if template { "template" } else { "page" }.to_owned()]
}

/// A page with its Markdown.
pub fn page(e: &TextEntry) -> Record {
    record(
        e.id.to_string(),
        &[
            ("path", text(&e.path)),
            ("name", text(&e.envelope.name)),
            ("content", text(&TextFormat::Markdown.export(&e.document))),
            ("kind", kind(e.template)),
            ("redirectTo", text(&e.redirect_to)),
            ("updatedBy", text(&e.updated_by)),
            ("revision", vec![e.revision.to_string()]),
        ],
    )
}

/// A search hit without the text (the caller reads the page when it wants all fields).
pub fn hit(h: &SearchHit) -> Record {
    record(
        h.id.to_string(),
        &[
            ("path", text(&h.path)),
            ("name", text(&h.name)),
            ("kind", kind(h.template)),
            ("updatedBy", text(&h.updated_by)),
        ],
    )
}

/// A space or folder.
pub fn folder(n: &Node) -> Record {
    let kind = if n.kind == NodeKind::Space {
        "space"
    } else {
        "folder"
    };
    record(
        n.id.to_string(),
        &[
            ("kind", vec![kind.to_owned()]),
            ("name", text(&n.name)),
            ("path", text(&n.path)),
            (
                "parent",
                n.parent.map(|p| p.to_string()).into_iter().collect(),
            ),
            ("position", vec![n.position.to_string()]),
        ],
    )
}

/// The id of a revision: `<page id>#<number>`.
pub fn revision_id(i: &RevisionInfo) -> String {
    format!("{}#{}", i.text, i.number)
}

/// A revision.
pub fn revision(i: &RevisionInfo) -> Record {
    record(
        revision_id(i),
        &[
            ("page", vec![i.text.to_string()]),
            ("number", vec![i.number.to_string()]),
            ("revision", vec![i.revision.to_string()]),
            ("author", text(&i.author)),
            ("at", vec![i.at.to_rfc3339()]),
            ("size", vec![i.size.to_string()]),
            ("summary", text(&i.summary)),
            ("path", text(&i.path)),
            (
                "restoredFrom",
                if i.restored_from == 0 {
                    Vec::new()
                } else {
                    vec![i.restored_from.to_string()]
                },
            ),
        ],
    )
}

/// A trashed page.
pub fn trash(t: &TrashItem) -> Record {
    record(
        t.id.to_string(),
        &[
            ("path", text(&t.path)),
            ("name", text(&t.name)),
            ("deletedAt", vec![t.deleted_at.to_rfc3339()]),
            ("deletedBy", text(&t.deleted_by)),
            ("size", vec![t.size.to_string()]),
        ],
    )
}

/// A link of the table; the id is derived from the three parts that tell links apart.
pub fn link(l: &LinkEntry) -> Record {
    record(
        format!("{}|{}|{}", l.source, l.target, l.at),
        &[
            ("source", vec![l.source.to_string()]),
            ("target", text(&l.target)),
            ("at", text(&l.at)),
            ("title", text(&l.title)),
            (
                "internal",
                vec![if l.internal { "yes" } else { "no" }.to_owned()],
            ),
        ],
    )
}

/// An answer that names the record it created or changed.
pub fn outcome(id: ObjectId) -> Outcome {
    Outcome {
        id: Some(id.to_string()),
        values: Values::new(),
    }
}
