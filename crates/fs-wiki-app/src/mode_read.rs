//! The reading wiki (`--read`): several folders in ONE wiki, each below its own path prefix, shown read only.
//!
//! Pattern: Facade — connect, import the sources (`import_sources`), order the pages by the documentation's navigation
//! (`nav.rs`) and hand the wiki to the reader of `fs-components` without editing, export or drafts. The first cut of the
//! reading wiki: no history and no backlinks yet. The reader with editing (`--reader`, one folder) is `mode_reader.rs`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::nav::Nav;
use crate::store_path::{data_dir, origins_file};
use crate::{FileOrigins, Wiki, WikiSource, import::CHECK_LIMIT, import_sources, words};
use fs_components_workbenches::{Labels, Options, Picture, Step, launch, snapshot};
use fs_wiki_client::RpcTexts;

use crate::args::Args;

/// Run the reading wiki.
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
        origins_file(&data, &records_key(&args.sources))
    };
    let origins = FileOrigins::open(&records).map_err(|e| e.to_string())?;
    let wiki = Arc::new(Wiki::new(Arc::new(texts), Arc::new(origins)));
    let report = runtime.block_on(import_sources(wiki.as_ref(), &args.sources, CHECK_LIMIT));
    eprintln!(
        "{} sources: {} new, {} updated, {} unchanged, {} failed, {} conflict(s), {} gone, {} skipped, {} attachment(s) not stored, in {:.1?}",
        args.sources.len(),
        report.imported,
        report.updated,
        report.unchanged,
        report.failures.len(),
        report.conflicts.len(),
        report.gone.len(),
        report.skipped.len(),
        report.attachments.len(),
        started.elapsed()
    );
    if report.records_ignored > 0 {
        let count = report.records_ignored;
        eprintln!(
            "{}",
            words::say(words::RECORDS_IGNORED, &[("count", &count)])
        );
    }
    let (nav, notes) = Nav::read(&args.sources, args.nav.as_deref());
    for note in notes {
        eprintln!("fs-wiki: {note}");
    }
    let source = Arc::new(
        WikiSource::from_report(wiki.texts(), runtime.handle().clone(), report)
            .with_nav(nav)
            .read_only(),
    );
    let mut labels = Labels::default();
    labels.search = "Search pages".into();
    if let Some(file) = &args.snapshot {
        let mut picture = Picture::new(source, args.size.0, args.size.1);
        picture.labels = labels;
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
    launch(&options, source).map_err(|e| e.to_string())
}

/// What names the records file of a set of sources: the prefixes, and a hash of the folders.
fn records_key(sources: &[(String, PathBuf)]) -> PathBuf {
    let prefixes: Vec<&str> = sources.iter().map(|(p, _)| p.as_str()).collect();
    let folders: String = sources
        .iter()
        .map(|(_, d)| {
            d.canonicalize()
                .unwrap_or_else(|_| d.clone())
                .display()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let hash = crate::hash::digest(folders.as_bytes());
    PathBuf::from("/sources").join(format!(
        "{}-{}",
        prefixes.join("+").replace('/', "_"),
        &hash[..8]
    ))
}

/// `open:<path>`, `follow:<href>`, `back`, `search:<term>`, `scroll:<n>`.
fn step(text: &str) -> Option<Step> {
    match text.split_once(':') {
        Some(("open", p)) => Some(Step::Open(p.into())),
        Some(("follow", h)) => Some(Step::Follow(h.into())),
        Some(("search", t)) => Some(Step::Search(t.into())),
        Some(("scroll", n)) => n.parse().ok().map(Step::Scroll),
        None if text == "back" => Some(Step::Back),
        _ => {
            eprintln!("fs-wiki-app: ignoring unknown step {text:?}");
            None
        }
    }
}
