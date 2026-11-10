//! The values of texts 0.2 read back from the wire (and the few written to it).
//!
//! Pattern: Adapter — the inverse of `wire_history.rs`, `wire_nodes.rs` and `wire_search.rs` of `fs-wiki-server`:
//! `RevisionInfo`, `TrashItem`, `Node`, `SearchHit`, `Resolution` of the core from `texts.capnp`. A value this
//! version does not know (a node kind, an outcome) is an unreadable reply, never a guess. `SearchHit::relevance` is not on
//! the wire; it is 0 here (a client continues a search by id, see `v2.rs`).

use fs_interfaces::common_capnp::uuid;
use fs_interfaces::rpc::{from_millis, read_object_id, read_uuid, write_uuid};
use fs_interfaces::texts_capnp::{
    expect, node, revision_info, search_hit, search_query, trash_item, NodeKind as WireKind,
    Outcome as WireOutcome, Placement as WirePlacement, Tri as WireTri,
};
use fs_objects::ObjectId;
use fs_wiki_core::{
    Expect, Highlight, Node, NodeKind, Outcome, Placement, Result, RevisionInfo, SearchHit,
    SearchQuery, Snippet, TrashItem, Tri,
};

use crate::read::unreadable;

fn text(t: capnp::Result<capnp::text::Reader<'_>>) -> Result<String> {
    t.and_then(|t| Ok(t.to_string()?)).map_err(unreadable)
}

/// A zero id is none.
pub(crate) fn optional_id(r: uuid::Reader<'_>) -> Result<Option<ObjectId>> {
    read_uuid(r)
        .map(|_| read_object_id(r))
        .transpose()
        .map_err(unreadable)
}

pub(crate) fn write_expect(mut out: expect::Builder<'_>, e: Expect) {
    match e {
        Expect::Any => out.set_any(()),
        Expect::Revision(n) => out.set_revision(n),
    }
}

pub(crate) fn info(r: revision_info::Reader<'_>) -> Result<RevisionInfo> {
    Ok(RevisionInfo {
        text: read_object_id(r.get_text().map_err(unreadable)?).map_err(unreadable)?,
        number: r.get_number(),
        revision: r.get_revision(),
        author: text(r.get_author())?,
        at: from_millis(r.get_at().map_err(unreadable)?.get_unix_millis()).map_err(unreadable)?,
        size: r.get_size(),
        summary: text(r.get_summary())?,
        path: text(r.get_path())?,
        restored_from: r.get_restored_from(),
    })
}

pub(crate) fn trash_item(r: trash_item::Reader<'_>) -> Result<TrashItem> {
    Ok(TrashItem {
        id: read_object_id(r.get_id().map_err(unreadable)?).map_err(unreadable)?,
        path: text(r.get_path())?,
        name: text(r.get_name())?,
        deleted_at: from_millis(r.get_deleted_at().map_err(unreadable)?.get_unix_millis())
            .map_err(unreadable)?,
        deleted_by: text(r.get_deleted_by())?,
        size: r.get_size(),
    })
}

pub(crate) fn node(r: node::Reader<'_>) -> Result<Node> {
    let kind = match r.get_kind() {
        Ok(WireKind::Page) => NodeKind::Page,
        Ok(WireKind::Folder) => NodeKind::Folder,
        Ok(WireKind::Space) => NodeKind::Space,
        _ => return Err(unreadable("a node of a kind this client does not know")),
    };
    Ok(Node {
        id: read_object_id(r.get_id().map_err(unreadable)?).map_err(unreadable)?,
        kind,
        name: text(r.get_name())?,
        path: text(r.get_path())?,
        parent: optional_id(r.get_parent().map_err(unreadable)?)?,
        position: r.get_position(),
        template: r.get_template(),
        redirect: r.get_redirect(),
    })
}

pub(crate) fn placement(p: Placement) -> WirePlacement {
    match p {
        Placement::First => WirePlacement::First,
        Placement::Last => WirePlacement::Last,
        Placement::Before => WirePlacement::Before,
        Placement::After => WirePlacement::After,
    }
}

pub(crate) fn outcome(o: std::result::Result<WireOutcome, capnp::NotInSchema>) -> Result<Outcome> {
    match o {
        Ok(WireOutcome::Resolved) => Ok(Outcome::Resolved),
        Ok(WireOutcome::NotFound) => Ok(Outcome::NotFound),
        Ok(WireOutcome::Loop) => Ok(Outcome::Loop),
        Ok(WireOutcome::TooManyHops) => Ok(Outcome::TooManyHops),
        _ => Err(unreadable("a redirect outcome this client does not know")),
    }
}

fn tri(t: Tri) -> WireTri {
    match t {
        Tri::Any => WireTri::Any,
        Tri::Only => WireTri::Only,
        Tri::Exclude => WireTri::Exclude,
    }
}

pub(crate) fn write_query(mut out: search_query::Builder<'_>, q: &SearchQuery) {
    out.set_term(q.term.as_str());
    out.set_prefix(q.prefix);
    let mut f = out.init_filter();
    write_uuid(
        f.reborrow().init_space(),
        q.filter.space.map(|s| s.as_uuid()),
    );
    f.set_path_prefix(q.filter.path_prefix.as_str());
    f.set_template(tri(q.filter.template));
    f.set_redirect(tri(q.filter.redirect));
    f.set_author(q.filter.author.as_str());
    f.reborrow()
        .init_updated_from()
        .set_unix_millis(q.filter.updated_from_ms);
    f.init_updated_to().set_unix_millis(q.filter.updated_to_ms);
}

fn field(name: &str) -> Result<&'static str> {
    ["name", "path", "body"]
        .into_iter()
        .find(|f| *f == name)
        .ok_or_else(|| unreadable(format!("a snippet of the field {name:?}")))
}

pub(crate) fn hit(r: search_hit::Reader<'_>, relevance: u32) -> Result<SearchHit> {
    let mut snippets = Vec::new();
    for s in r.get_snippets().map_err(unreadable)? {
        let mut highlights = Vec::new();
        for h in s.get_highlights().map_err(unreadable)? {
            highlights.push(Highlight {
                start: h.get_start(),
                end: h.get_end(),
            });
        }
        snippets.push(Snippet {
            field: field(&text(s.get_field())?)?,
            text: text(s.get_text())?,
            highlights,
            cut_before: s.get_cut_before(),
            cut_after: s.get_cut_after(),
        });
    }
    Ok(SearchHit {
        id: read_object_id(r.get_id().map_err(unreadable)?).map_err(unreadable)?,
        path: text(r.get_path())?,
        name: text(r.get_name())?,
        updated: from_millis(r.get_updated().map_err(unreadable)?.get_unix_millis())
            .map_err(unreadable)?,
        updated_by: text(r.get_updated_by())?,
        template: r.get_template(),
        redirect: r.get_redirect(),
        snippets,
        relevance,
    })
}
