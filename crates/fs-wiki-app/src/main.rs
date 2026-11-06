//! The binary: wiring only.
//!
//! Pattern: Facade consumer — parse arguments, open the store, import, hand the wiki to the reader.

use std::sync::Arc;
use std::time::Instant;

use fs_components_workbenches::{DraftStore, Labels, Options, Picture, Step, launch, snapshot};
use fs_store::{Store, TursoStore};
use fs_wiki_app::args::{HELP, StoreChoice, parse};
use fs_wiki_app::store_path::{data_dir, store_file};
use fs_wiki_app::{FileDraftStore, WikiSource, import_folder};
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
    let file = match &args.store {
        StoreChoice::Memory => None,
        StoreChoice::File(file) => Some(file.clone()),
        StoreChoice::Default => {
            let data = data_dir()
                .ok_or("no data directory (set HOME or XDG_DATA_HOME, or use --store)")?;
            Some(store_file(&data, &args.docs))
        }
    };
    if let Some(dir) = file.as_ref().and_then(|f| f.parent()) {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    match &file {
        Some(f) => eprintln!("store: {}", f.display()),
        None => eprintln!("store: in memory, nothing is kept after exit"),
    }
    let store: Arc<dyn Store> = runtime
        .block_on(async {
            match &file {
                Some(file) => TursoStore::open(file).await,
                None => TursoStore::open_in_memory().await,
            }
        })
        .map(|s| Arc::new(s) as Arc<dyn Store>)
        .map_err(|e| format!("cannot open the store: {e}"))?;
    let wiki = Arc::new(WikiRepo::new(store, "https://localhost/actors/reader"));
    let report = runtime.block_on(import_folder(wiki.as_ref(), &args.docs));
    eprintln!(
        "{} new, {} updated, {} unchanged, {} failed, {} conflict(s), {} gone, {} skipped, in {:.1?}",
        report.imported,
        report.updated,
        report.unchanged,
        report.failures.len(),
        report.conflicts.len(),
        report.gone.len(),
        report.skipped.len(),
        started.elapsed()
    );
    let source = Arc::new(
        WikiSource::from_report(wiki.clone(), runtime.handle().clone(), report)
            .with_origins(wiki, args.docs.clone()),
    );
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
    let drafts = args
        .drafts
        .clone()
        .or_else(fs_wiki_app::drafts::default_dir)
        .map(|dir| FileDraftStore::new(dir, &args.docs))
        .transpose()?
        .map(|s| Arc::new(s) as Arc<dyn DraftStore>);
    let mut options = Options::default();
    options.drafts = drafts;
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
