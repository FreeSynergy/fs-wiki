//! The page tree shows the new title after a save (the title of a page is its first heading, written in the editor).
//!
//! Pattern: Observation on the picture of the real window (`AppWidget::picture`): the sidebar is drawn before and after a
//! save through the reader; the tree is made again by a thread after the save, so the picture is taken until the sidebar
//! changes (at most a few seconds).

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fs_components_app::AppWidget;
use fs_components_widgets::Document;
use fs_components_workbenches::{
    DocumentSource, EditableText, Entry, Hit, Labels, LinkTarget, ReaderWidget, SourceError,
    WikiLabels,
};
use fs_wiki_app::frontend::Frontend;
use fs_wiki_app::frontend_window::{Setup, window};
use fs_wiki_app::source::{CreateError, PageMaker, SavedHook};

/// One page whose title is its text without the `# `; saving calls the hook as `WikiSource` does.
struct One {
    text: Mutex<String>,
    hook: Mutex<Option<SavedHook>>,
    call_hook: bool,
}

impl DocumentSource for One {
    fn entries(&self, _prefix: &str) -> Result<Vec<Entry>, SourceError> {
        let title = self
            .text
            .lock()
            .unwrap()
            .trim_start_matches("# ")
            .trim()
            .to_owned();
        Ok(vec![Entry::new("a.md", title)])
    }
    fn read(&self, path: &str) -> Result<Document, SourceError> {
        Ok(Document::new(path, Vec::new()))
    }
    fn resolve(&self, _from: &str, href: &str) -> LinkTarget {
        LinkTarget::Missing(href.into())
    }
    fn find(&self, _term: &str) -> Result<Vec<Hit>, SourceError> {
        Ok(Vec::new())
    }
    fn can_edit(&self, _path: &str) -> bool {
        true
    }
    fn source_text(&self, _path: &str) -> Result<EditableText, SourceError> {
        Ok(EditableText::new(self.text.lock().unwrap().clone(), None))
    }
    fn save(&self, _path: &str, text: &str, _revision: Option<&str>) -> Result<(), SourceError> {
        text.clone_into(&mut self.text.lock().unwrap());
        let hook = self.hook.lock().unwrap().clone();
        if let (true, Some(hook)) = (self.call_hook, hook) {
            hook();
        }
        Ok(())
    }
}

impl PageMaker for One {
    fn can_create(&self) -> bool {
        true
    }
    fn create_page(&self, _parent: Option<&str>, _title: &str) -> Result<String, CreateError> {
        Err(CreateError::ReadOnly)
    }
    fn on_saved(&self, hook: SavedHook) {
        *self.hook.lock().unwrap() = Some(hook);
    }
}

/// The pixels of the sidebar (the first 298 columns, the rows of the page list) of a picture of `app`.
fn sidebar(app: &AppWidget, file: &std::path::Path) -> Vec<u8> {
    app.picture(1000, 650, file).expect("picture");
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(file).unwrap()));
    let mut reading = decoder.read_info().unwrap();
    let mut pixels = vec![0; reading.output_buffer_size().unwrap()];
    let info = reading.next_frame(&mut pixels).unwrap();
    let step = info.color_type.samples();
    let mut out = Vec::new();
    for row in 150..260_u32 {
        let from = (row * info.width) as usize * step;
        out.extend_from_slice(&pixels[from..from + 298 * step]);
    }
    out
}

fn saved_changes_the_sidebar(call_hook: bool) -> bool {
    let one = Arc::new(One {
        text: Mutex::new("# A\n".into()),
        hook: Mutex::new(None),
        call_hook,
    });
    let reader = ReaderWidget::new(one.clone(), Labels::default(), None);
    let setup = Setup {
        labels: WikiLabels::default(),
        german: false,
        bare: Arc::new(AtomicBool::new(false)),
        settings: None,
        pages: Some(one),
    };
    let app = window("Wiki", reader.clone(), Frontend::Confluence, setup).expect("window");
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("w.png");
    let before = sidebar(&app, &file);
    reader.with_reader(|r| {
        r.begin_edit();
        r.set_draft("# A much longer title\n");
        r.save();
    });
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs(4) {
        std::thread::sleep(Duration::from_millis(200));
        if sidebar(&app, &file) != before {
            return true;
        }
    }
    false
}

#[test]
fn the_tree_shows_the_title_that_was_saved() {
    assert!(
        saved_changes_the_sidebar(true),
        "the sidebar did not change after the save"
    );
}

#[test]
fn without_the_hook_the_tree_stays_as_it_was() {
    // The counter-proof of the test above, kept as a test: the same run, the source does not tell the window.
    assert!(
        !saved_changes_the_sidebar(false),
        "the sidebar changed without the hook"
    );
}
