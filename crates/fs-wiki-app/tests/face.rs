//! The face of the reading wiki: remembered in a file, chosen in the menu View > Face.
//!
//! Pattern: Integration tests over the public API; the menu entry is chosen as a press on its line does (`choose`), with a
//! small source of two pages in place of the wiki.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use fs_components_app::AppWidget;
use fs_components_widgets::Document;
use fs_components_workbenches::{
    DocumentSource, Entry, Hit, Labels, LinkTarget, ReaderWidget, SourceError, WikiLabels,
};
use fs_wiki_app::face::{self, Face};
use fs_wiki_app::face_window::{FACE_PREFIX, QUIT, Setup, menus, window};

struct Two;

impl DocumentSource for Two {
    fn entries(&self, _prefix: &str) -> Result<Vec<Entry>, SourceError> {
        Ok(vec![Entry::new("a", "A"), Entry::new("b", "B")])
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
}

fn app(
    face: Face,
    settings: Option<std::path::PathBuf>,
    german: bool,
) -> (AppWidget, Arc<AtomicBool>) {
    let bare = Arc::new(AtomicBool::new(false));
    let reader = ReaderWidget::new(Arc::new(Two), Labels::default(), None);
    let setup = Setup {
        labels: WikiLabels::default(),
        german,
        bare: Arc::clone(&bare),
        settings,
    };
    (window("Wiki", reader, face, setup).expect("window"), bare)
}

fn lines(app: &AppWidget) -> Vec<(String, String)> {
    app.with_shell(|s| {
        s.menus()
            .iter()
            .flat_map(|m| m.entries.iter().map(|e| (e.id.clone(), e.label.clone())))
            .collect()
    })
}

#[test]
fn a_new_installation_shows_confluence() {
    let dir = tempfile::tempdir().unwrap();
    let file = face::settings_file(dir.path());
    assert_eq!(face::load(&file), None);
    assert_eq!(Face::DEFAULT, Face::Confluence);
}

#[test]
fn a_face_is_saved_and_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let file = face::settings_file(dir.path());
    for f in Face::ALL {
        face::save(&file, f).unwrap();
        assert_eq!(face::load(&file), Some(f));
    }
    let left = std::fs::read_dir(file.parent().unwrap()).unwrap().count();
    assert_eq!(left, 1, "no temporary file is left behind");
}

#[test]
fn a_broken_file_means_the_default() {
    let dir = tempfile::tempdir().unwrap();
    let file = face::settings_file(dir.path());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    for broken in ["", "notion", "\u{0}\u{1}", "a b", "classic;"] {
        std::fs::write(&file, broken).unwrap();
        assert_eq!(face::load(&file), None, "{broken:?}");
    }
    std::fs::write(&file, [0xff_u8, 0xfe]).unwrap();
    assert_eq!(face::load(&file), None);
}

#[test]
fn the_menu_view_has_the_three_faces_and_marks_the_shown_one() {
    let (app, _) = app(Face::Confluence, None, false);
    let got = lines(&app);
    assert_eq!(
        got,
        [
            (QUIT.to_owned(), "Quit".to_owned()),
            ("face-classic".into(), "   Face: Classic".into()),
            ("face-a".into(), "✓ Face: Confluence".into()),
            ("face-b".into(), "   Face: Notion".into()),
        ]
    );
    let german = menus(Face::Notion, true);
    assert_eq!(german[1].label, "Ansicht");
    assert_eq!(german[1].entries[1].label, "   Gesicht: Confluence");
    assert_eq!(german[1].entries[1].id, format!("{FACE_PREFIX}a"));
    assert_eq!(german[1].entries[2].label, "✓ Gesicht: Notion");
}

#[test]
fn choosing_a_face_switches_marks_and_remembers() {
    let dir = tempfile::tempdir().unwrap();
    let file = face::settings_file(dir.path());
    let (app, bare) = app(Face::Confluence, Some(file.clone()), false);
    assert!(bare.load(Ordering::Relaxed));
    assert!(app.choose("face-b"));
    assert_eq!(face::load(&file), Some(Face::Notion));
    assert!(lines(&app).contains(&("face-b".into(), "✓ Face: Notion".into())));
    assert!(app.choose("face-classic"));
    assert_eq!(face::load(&file), Some(Face::Classic));
    assert!(
        !bare.load(Ordering::Relaxed),
        "the plain face has the language bar in the page"
    );
    assert!(app.choose("face-a"));
    assert!(bare.load(Ordering::Relaxed));
    assert_eq!(face::load(&file), Some(Face::Confluence));
    assert!(!app.choose("face-x"), "an unknown entry does nothing");
}

#[test]
fn a_run_with_an_override_remembers_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (app, _) = app(Face::Notion, None, false);
    assert!(app.choose("face-classic"));
    assert!(!face::settings_file(dir.path()).exists());
}

#[test]
fn the_frontend_option_takes_classic_a_and_b() {
    let args = |words: &[&str]| {
        let words: Vec<String> = words.iter().map(|w| (*w).to_owned()).collect();
        fs_wiki_app::args::parse(&words).unwrap()
    };
    assert_eq!(
        args(&["--frontend", "classic"]).frontend.as_deref(),
        Some("classic")
    );
    assert_eq!(args(&["--frontend", "b"]).frontend.as_deref(), Some("b"));
    assert!(fs_wiki_app::args::parse(&["--frontend".into(), "c".into()]).is_err());
    assert_eq!(Face::parse("classic"), Some(Face::Classic));
    assert_eq!(Face::parse("c"), None);
}
