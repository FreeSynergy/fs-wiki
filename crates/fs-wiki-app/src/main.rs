//! The binary: wiring only.
//!
//! Pattern: Facade — the one entry point over the app: parse arguments, connect to the server,
//! import, hand the wiki to the reader.

use std::sync::Arc;
use std::time::Instant;

use fs_components_workbenches::{DraftStore, Labels, Options, Picture, Step, launch, snapshot};
use fs_wiki_app::args::{HELP, parse};
use fs_wiki_app::server::{connect, socket_path};
use fs_wiki_app::store_path::{data_dir, old_store_file, origins_file};
use fs_wiki_app::{Conflict, FileDraftStore, FileOrigins, Wiki, WikiSource, import_folder, words};
use fs_wiki_core::Texts;

fn main() {
    if let Err(message) = run() {
        eprintln!("fs-wiki-app: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let given: Vec<String> = std::env::args().skip(1).collect();
    let args = parse(&given)?;
    if args.help {
        println!("{HELP}");
        return Ok(());
    }
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let started = Instant::now();
    let socket = socket_path(args.socket.as_deref())?;
    let texts = connect(&socket)?;
    let records = if let Some(file) = &args.origins {
        file.clone()
    } else {
        let data =
            data_dir().ok_or("no data directory (set HOME or XDG_DATA_HOME, or use --origins)")?;
        if let Some(old) = old_store_file(&data, &args.docs) {
            eprintln!(
                "{}",
                words::say(words::OLD_STORE, &[("path", &old.display())])
            );
        }
        origins_file(&data, &args.docs)
    };
    eprintln!(
        "server: {}, records: {}",
        socket.display(),
        records.display()
    );
    let origins = FileOrigins::open(&records).map_err(|e| e.to_string())?;
    let wiki = Arc::new(Wiki::new(Arc::new(texts), Arc::new(origins)));
    let report = runtime.block_on(import_folder(wiki.as_ref(), &args.docs));
    print_summary(&report, started);
    let drafts = args
        .drafts
        .clone()
        .or_else(fs_wiki_app::drafts::default_dir)
        .map(|dir| FileDraftStore::new(dir, &args.docs))
        .transpose()?;
    let mut report = report;
    if let Some(store) = &drafts {
        // A draft of a page that moved or was deleted is not offered by path any more: show it.
        let handle = runtime.handle().clone();
        let texts = wiki.clone();
        let exists = move |p: &str| matches!(handle.block_on(texts.get_by_path(p)), Ok(Some(_)));
        for d in store.orphans(&exists).map_err(|e| e.message)? {
            report.conflicts.push(Conflict {
                file: d.path,
                text_path: None,
                file_text: d.text,
                reason:
                    "an unsaved draft of a page that is no longer at this path (moved or deleted)"
                        .into(),
            });
        }
    }
    let drafts = drafts.map(|s| Arc::new(s) as Arc<dyn DraftStore>);
    let source = Arc::new(
        WikiSource::from_report(wiki.texts(), runtime.handle().clone(), report)
            .with_origins(wiki.origins_store(), args.docs.clone()),
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
    let mut options = Options::default();
    options.drafts = drafts;
    options.title = "FreeSynergy Wiki".into();
    options.labels = labels;
    options.export_target = export_target;
    launch(&options, source).map_err(|e| e.to_string())
}

/// One line of counts, then a word for what was ignored or is missing.
fn print_summary(report: &fs_wiki_app::Report, started: Instant) {
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
    if report.records_ignored > 0 {
        let count = report.records_ignored;
        eprintln!(
            "{}",
            words::say(words::RECORDS_IGNORED, &[("count", &count)])
        );
    }
    if !report.missing.is_empty() {
        let count = report.missing.len();
        eprintln!(
            "{}",
            words::say(words::MISSING_SUMMARY, &[("count", &count)])
        );
    }
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
