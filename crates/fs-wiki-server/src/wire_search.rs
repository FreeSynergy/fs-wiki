//! A search and a redirect resolution on the wire.
//!
//! Pattern: Adapter — the query is read with its limits left to the provider (it answers
//! `invalid` with one issue per reason); hits, snippets and highlights are written as they are.

use fs_interfaces::rpc::{from_millis, to_millis, write_object_id};
use fs_interfaces::texts_capnp::{
    search_hit, search_query, Outcome as WireOutcome, Tri as WireTri,
};
use fs_wiki_core::{Outcome, SearchFilter, SearchHit, SearchQuery, Tri};

use crate::errors::invalid;
use crate::wire::text_arg;
use crate::wire_history::ROW_BYTES;
use crate::wire_nodes::optional_id;

fn tri(t: Result<WireTri, capnp::NotInSchema>) -> capnp::Result<Tri> {
    match t {
        Ok(WireTri::Any) => Ok(Tri::Any),
        Ok(WireTri::Only) => Ok(Tri::Only),
        Ok(WireTri::Exclude) => Ok(Tri::Exclude),
        Err(_) => Err(invalid("a filter is any, only or exclude", "filter")),
    }
}

pub(crate) fn read_query(q: search_query::Reader<'_>) -> capnp::Result<SearchQuery> {
    let f = q.get_filter()?;
    let at = |t: fs_interfaces::common_capnp::timestamp::Reader<'_>| -> capnp::Result<i64> {
        let ms = t.get_unix_millis();
        from_millis(ms).map(to_millis)
    };
    Ok(SearchQuery {
        term: text_arg(q.get_term()?)?.to_string(),
        prefix: q.get_prefix(),
        filter: SearchFilter {
            space: optional_id(f.get_space()?)?,
            path_prefix: text_arg(f.get_path_prefix()?)?.to_string(),
            template: tri(f.get_template())?,
            redirect: tri(f.get_redirect())?,
            author: text_arg(f.get_author()?)?.to_string(),
            updated_from_ms: at(f.get_updated_from()?)?,
            updated_to_ms: at(f.get_updated_to()?)?,
        },
    })
}

pub(crate) fn outcome(o: Outcome) -> WireOutcome {
    match o {
        Outcome::Resolved => WireOutcome::Resolved,
        Outcome::NotFound => WireOutcome::NotFound,
        Outcome::Loop => WireOutcome::Loop,
        Outcome::TooManyHops => WireOutcome::TooManyHops,
    }
}

pub(crate) fn write_hit(mut out: search_hit::Builder<'_>, h: &SearchHit) {
    write_object_id(out.reborrow().init_id(), h.id);
    out.set_path(h.path.as_str());
    out.set_name(h.name.as_str());
    out.reborrow()
        .init_updated()
        .set_unix_millis(to_millis(h.updated));
    out.set_updated_by(h.updated_by.as_str());
    out.set_template(h.template);
    out.set_redirect(h.redirect);
    let mut list = out.init_snippets(u32::try_from(h.snippets.len()).unwrap_or(0));
    for (i, s) in (0u32..).zip(&h.snippets) {
        let mut w = list.reborrow().get(i);
        w.set_field(s.field);
        w.set_text(s.text.as_str());
        w.set_cut_before(s.cut_before);
        w.set_cut_after(s.cut_after);
        let mut marks = w.init_highlights(u32::try_from(s.highlights.len()).unwrap_or(0));
        for (j, m) in (0u32..).zip(&s.highlights) {
            let mut mark = marks.reborrow().get(j);
            mark.set_start(m.start);
            mark.set_end(m.end);
        }
    }
}

pub(crate) fn hit_weight(h: &SearchHit) -> usize {
    ROW_BYTES
        + h.path.len()
        + h.name.len()
        + h.updated_by.len()
        + h.snippets
            .iter()
            .map(|s| s.text.len() + 64 + 8 * s.highlights.len())
            .sum::<usize>()
}
