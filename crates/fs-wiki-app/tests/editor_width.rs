//! The editor takes the width of its column in every frontend (finding of 2026-11-13: a 312 px text area in a 430 px column).
//!
//! Pattern: Measurement on the picture of the real window (`AppWidget::picture`: title bar, styles of the app, the layouts of
//! `fs-components`): the editing page is drawn, the PNG is read, and the outer lines of the editor on a row through it are
//! measured against the column. The picture is the same code path as the window's, drawn offscreen.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use fs_components_app::AppWidget;
use fs_components_widgets::Document;
use fs_components_workbenches::{
    DocumentSource, EditableText, Entry, Hit, Labels, LinkTarget, Reader, ReaderWidget, SourceError,
    WikiLabels,
};
use fs_wiki_app::frontend::Frontend;
use fs_wiki_app::frontend_window::{Setup, window};

struct One(Mutex<String>);

impl DocumentSource for One {
    fn entries(&self, _prefix: &str) -> Result<Vec<Entry>, SourceError> {
        Ok(vec![Entry::new("a.md", "A")])
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
        Ok(EditableText::new(self.0.lock().unwrap().clone(), None))
    }
}

/// The picture of the editing window in `frontend`: (width, left, right) of the editor's outer lines on a row through its
/// lower part, searched between `from` and `to` (the column), as pixels.
fn measure(frontend: Frontend, from: u32, to: u32, rows: std::ops::Range<u32>) -> (u32, u32, u32) {
    let source = Arc::new(One(Mutex::new("# A\n".into())));
    let reader = ReaderWidget::new(source, Labels::default(), None);
    let setup = Setup {
        labels: WikiLabels::default(),
        german: false,
        bare: Arc::new(AtomicBool::new(false)),
        settings: None,
        pages: None,
    };
    let app: AppWidget = window("Wiki", reader.clone(), frontend, setup).expect("window");
    reader.with_reader(Reader::begin_edit);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("w.png");
    app.picture(1000, 650, &file).expect("picture");
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&file).unwrap()));
    let mut reading = decoder.read_info().unwrap();
    let mut pixels = vec![0; reading.output_buffer_size().unwrap()];
    let info = reading.next_frame(&mut pixels).unwrap();
    let step = info.color_type.samples();
    let dark = |x: u32, row: u32| {
        let at = (row * info.width + x) as usize * step;
        pixels[at..at + 3].iter().any(|c| *c < 235)
    };
    // The widest row: its first and last dark pixel (in the editor's rows these are its outer lines).
    rows.filter_map(|row| {
        let left = (from..to).find(|x| dark(*x, row))?;
        let right = (from..to).rev().find(|x| dark(*x, row))?;
        Some((right - left, left, right))
    })
    .max()
    .expect("something is drawn")
}

#[test]
fn the_editor_fills_the_column_of_confluence_notion_and_classic() {
    // Confluence: the column is 300..730 (tree 300 px, outline 270 px); the row is inside the text area, below its text.
    let (width, ..) = measure(Frontend::Confluence, 302, 728, 450..451);
    assert!(width >= 390, "Confluence: {width} px of 428");
    // Classic: the column is 320..1000.
    let (width, ..) = measure(Frontend::Classic, 321, 1000, 450..451);
    assert!(width >= 650, "Classic: {width} px of 679");
}

#[test]
fn the_blocks_of_notion_use_the_reading_column() {
    // Notion: the page is 266..1000 wide; the block of the text is a box in its reading column.
    let (width, ..) = measure(Frontend::Notion, 266, 998, 110..600);
    assert!(width >= 600, "Notion: {width} px of 732");
}
