//! The binary: wiring only.
//!
//! Pattern: Facade consumer — parse arguments, open the store, import, hand the wiki to the reader.

use std::sync::Arc;
use std::time::Instant;

use fs_components_workbenches::{Labels, Options, Picture, Step, launch, snapshot};
use fs_store::{Store, TursoStore};
use fs_wiki_app::args::{HELP, parse};
use fs_wiki_app::{WikiSource, import_folder};
use fs_wiki_core::WikiRepo;

fn main() {
    if let Err(message) = run() {
        eprintln!("fs-wiki-app: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = parse(&argv)?;
    if args.help {
        println!("{HELP}");
        return Ok(());
    }
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let started = Instant::now();
    let store: Arc<dyn Store> = runtime
        .block_on(async {
            match &args.store {
                Some(file) => TursoStore::open(file).await,
                None => TursoStore::open_in_memory().await,
            }
        })
        .map(|s| Arc::new(s) as Arc<dyn Store>)
        .map_err(|e| format!("cannot open the store: {e}"))?;
    let wiki = Arc::new(WikiRepo::new(store, "https://localhost/actors/reader"));
    let report = runtime.block_on(import_folder(wiki.as_ref(), &args.docs));
    eprintln!(
        "imported {} page(s), {} failed, in {:.1?}",
        report.imported,
        report.failures.len(),
        started.elapsed()
    );
    let source = Arc::new(WikiSource::new(
        wiki,
        runtime.handle().clone(),
        report.failures,
    ));
    let export_target = args.export.as_ref().map(|p| p.display().to_string());
    let mut labels = Labels::default();
    labels.search = "Search pages".into();
    if let Some(file) = &args.snapshot {
        let mut picture = Picture::new(source, args.size.0, args.size.1);
        picture.labels = labels;
        picture.export_target = export_target;
        picture.steps = args.steps.iter().filter_map(|s| step(s)).collect();
        let shot = snapshot(picture, file).map_err(|e| e.to_string())?;
        eprintln!(
            "wrote {}, showing {:?}",
            file.display(),
            shot.reader.current_path()
        );
        return Ok(());
    }
    let mut options = Options::default();
    options.title = "FreeSynergy Wiki".into();
    options.labels = labels;
    options.export_target = export_target;
    launch(&options, source).map_err(|e| e.to_string())
}

/// `open:<path>`, `follow:<href>`, `back`, `search:<term>`.
fn step(text: &str) -> Option<Step> {
    match text.split_once(':') {
        Some(("open", p)) => Some(Step::Open(p.into())),
        Some(("follow", h)) => Some(Step::Follow(h.into())),
        Some(("search", t)) => Some(Step::Search(t.into())),
        Some(("type", t)) => Some(Step::Type(t.replace("\\n", "\n"))),
        None if text == "back" => Some(Step::Back),
        None if text == "edit" => Some(Step::Edit),
        None if text == "save" => Some(Step::Save),
        None if text == "cancel" => Some(Step::Cancel),
        Some(("scroll", n)) => n.parse().ok().map(Step::Scroll),
        None if text == "export" => Some(Step::Export),
        _ => {
            eprintln!("fs-wiki-app: ignoring unknown step {text:?}");
            None
        }
    }
}
