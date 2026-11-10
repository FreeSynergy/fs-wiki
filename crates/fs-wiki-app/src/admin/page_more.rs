//! `Page` commands that read: `resolve` and `export`.
//!
//! Pattern: Strategy by command, as `page.rs` (split for the file limit); their results are the result fields of the
//! description.

use fs_admin_frontend::{BackendError, Outcome, Values};
use fs_wiki_core::Outcome as Resolved;

use super::ctx::{id, need, unsupported, Ctx};

fn word(o: Resolved) -> &'static str {
    match o {
        Resolved::Resolved => "resolved",
        Resolved::NotFound => "not-found",
        Resolved::Loop => "loop",
        Resolved::TooManyHops => "too-many-hops",
    }
}

/// `resolve` and `export`; any other name is not carried out.
pub fn run(ctx: &Ctx, name: &str, input: &Values) -> Result<Outcome, BackendError> {
    match name {
        "resolve" => {
            let r = ctx.block(ctx.texts.resolve(need(input, "path")?))?;
            let mut values = Values::new();
            values.insert("outcome".into(), vec![word(r.outcome).to_owned()]);
            values.insert("hops".into(), r.hops);
            let target = r.text.as_ref().map(|t| t.id.to_string());
            if let Some(t) = &target {
                values.insert("id".into(), vec![t.clone()]);
            }
            Ok(Outcome { id: target, values })
        }
        "export" => {
            let text = ctx.block(ctx.texts.export_text(id(input, "id")?, need(input, "mediaType")?))?;
            let mut values = Values::new();
            values.insert("text".into(), vec![text]);
            Ok(Outcome { id: None, values })
        }
        other => Err(unsupported(other)),
    }
}
