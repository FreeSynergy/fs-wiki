//! The Admin mode: the standard window with the generated frontend, or its headless check.
//!
//! Pattern: Facade — ask the (connected) server for its type descriptions, then open the window (`fs_wiki_app::admin`)
//! or, with `--check`, list through the same backends without drawing anything.

use std::sync::Arc;

use fs_admin_frontend::Values;
use fs_wiki_client::RpcTexts;

use crate::admin::{backend_for, open, Ctx};
use crate::args::Args;

/// Run the Admin mode.
///
/// # Errors
/// A message for the person.
pub fn run(args: &Args, texts: RpcTexts) -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let texts = Arc::new(texts);
    let registry = runtime.block_on(texts.types()).map_err(|e| format!("the server gave no type description: {e}"))?;
    let ctx = Ctx::new(texts, runtime.handle().clone());
    if args.check {
        return check(&registry, &ctx);
    }
    open(&registry, &ctx, "source").map_err(|e| e.to_string())
}

/// One line per data type: how many records its list has; no window.
fn check(registry: &fs_types::TypeRegistry, ctx: &Ctx) -> Result<(), String> {
    for t in registry.iter() {
        let line = match backend_for(&t.name, ctx) {
            Some(backend) => match backend.list(&Values::new()) {
                Ok(records) => format!("{} records", records.len()),
                Err(e) => format!("list: {e:?}"),
            },
            None => "no backend".to_owned(),
        };
        let open = t.commands.iter().filter(|c| !fs_wiki_core::describe::is_supported(&t.name, &c.name)).count();
        println!("{}: {} commands ({open} still to come), {line}", t.name, t.commands.len());
    }
    Ok(())
}
