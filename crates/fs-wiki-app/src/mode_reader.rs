//! The reader mode (`--reader`): the documentation reader and editor on the workbench of `fs-components`.
//!
//! Pattern: Facade — connect to the server, import a folder, hand the wiki to the reader. The Admin window (default)
//! is `admin.rs`; this is the reading and editing view of the documentation folder, kept because the Admin frontend
//! has no reader (named in `DESIGN.md`).

use std::sync::Arc;
use std::time::Instant;

use crate::store_path::{data_dir, old_store_file, origins_file};
use crate::{Conflict, FileDraftStore, FileOrigins, Wiki, WikiSource, import_folder, words};
use fs_components_workbenches::{DraftStore, Labels, Options, Picture, Step, launch, snapshot};
use fs_wiki_client::RpcTexts;
use fs_wiki_core::Texts;

use crate::args::Args;

/// Run the reader.
///
/// # Errors
/// A message for the person.
pub fn run(args: &Args, texts: RpcTexts) -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let started = Instant::now();
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
    eprintln!("records: {}", records.display());
    let origins = FileOrigins::open(&records).map_err(|e| e.to_string())?;
    let wiki = Arc::new(Wiki::new(Arc::new(texts), Arc::new(origins)));
    let report = runtime.block_on(import_folder(wiki.as_ref(), &args.docs));
    print_summary(&report, started);
    let drafts = args
        .drafts
        .clone()
        .or_else(crate::drafts::default_dir)
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
fn print_summary(report: &crate::Report, started: Instant) {
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
