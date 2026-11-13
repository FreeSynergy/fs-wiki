//! The frontend of the reading wiki: remembered in a file, chosen in the menu View > Frontend.
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
use fs_wiki_app::frontend::{self, Frontend};
use fs_wiki_app::frontend_window::{FRONTEND_PREFIX, QUIT, Setup, menus, window};

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
    shown: Frontend,
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
        pages: None,
    };
    (window("Wiki", reader, shown, setup).expect("window"), bare)
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
    let file = frontend::settings_file(dir.path());
    assert_eq!(frontend::load(&file), None);
    assert_eq!(Frontend::DEFAULT, Frontend::Confluence);
}

#[test]
fn a_frontend_is_saved_and_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let file = frontend::settings_file(dir.path());
    for f in Frontend::ALL {
        frontend::save(&file, f).unwrap();
        assert_eq!(frontend::load(&file), Some(f));
    }
    let left = std::fs::read_dir(file.parent().unwrap()).unwrap().count();
    assert_eq!(left, 1, "no temporary file is left behind");
}

#[test]
fn a_broken_file_means_the_default() {
    let dir = tempfile::tempdir().unwrap();
    let file = frontend::settings_file(dir.path());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    for broken in ["", "notion", "\u{0}\u{1}", "a b", "classic;"] {
        std::fs::write(&file, broken).unwrap();
        assert_eq!(frontend::load(&file), None, "{broken:?}");
    }
    std::fs::write(&file, [0xff_u8, 0xfe]).unwrap();
    assert_eq!(frontend::load(&file), None);
}

#[test]
fn the_choice_of_an_earlier_version_is_taken_over_once_and_the_old_file_stays() {
    let dir = tempfile::tempdir().unwrap();
    let file = frontend::settings_file(dir.path());
    let old = file.with_file_name("face");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&old, "b\n").unwrap();
    assert_eq!(frontend::load(&file), Some(Frontend::Notion));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "b\n", "written");
    assert_eq!(std::fs::read_to_string(&old).unwrap(), "b\n", "kept");
    // From now on the new file counts, also when the old one says something else.
    std::fs::write(&old, "classic\n").unwrap();
    assert_eq!(frontend::load(&file), Some(Frontend::Notion));
    // A broken new file is the default; the old one is not asked then.
    std::fs::write(&file, "nonsense").unwrap();
    assert_eq!(frontend::load(&file), None);
}

#[test]
fn a_broken_old_file_is_not_taken_over() {
    let dir = tempfile::tempdir().unwrap();
    let file = frontend::settings_file(dir.path());
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file.with_file_name("face"), "no such face").unwrap();
    assert_eq!(frontend::load(&file), None);
    assert!(!file.exists(), "nothing was written");
}

#[test]
fn the_menu_view_has_the_three_frontends_and_marks_the_shown_one() {
    let (app, _) = app(Frontend::Confluence, None, false);
    let got = lines(&app);
    assert_eq!(
        got,
        [
            (QUIT.to_owned(), "Quit".to_owned()),
            ("frontend-classic".into(), "   Frontend: Classic".into()),
            ("frontend-a".into(), "✓ Frontend: Confluence".into()),
            ("frontend-b".into(), "   Frontend: Notion".into()),
        ]
    );
    let german = menus(Frontend::Notion, true);
    assert_eq!(german[1].label, "Ansicht");
    assert_eq!(german[1].entries[1].label, "   Frontend: Confluence");
    assert_eq!(german[1].entries[1].id, format!("{FRONTEND_PREFIX}a"));
    assert_eq!(german[1].entries[2].label, "✓ Frontend: Notion");
    for menu in menus(Frontend::Classic, true)
        .iter()
        .chain(menus(Frontend::Classic, false).iter())
    {
        let words = std::iter::once(&menu.label).chain(menu.entries.iter().map(|e| &e.label));
        for word in words {
            assert!(
                !word.contains("Gesicht") && !word.to_lowercase().contains("face"),
                "{word}"
            );
        }
    }
}

#[test]
fn choosing_a_frontend_switches_marks_and_remembers() {
    let dir = tempfile::tempdir().unwrap();
    let file = frontend::settings_file(dir.path());
    let (app, bare) = app(Frontend::Confluence, Some(file.clone()), false);
    assert!(bare.load(Ordering::Relaxed));
    assert!(app.choose("frontend-b"));
    assert_eq!(frontend::load(&file), Some(Frontend::Notion));
    assert!(lines(&app).contains(&("frontend-b".into(), "✓ Frontend: Notion".into())));
    assert!(app.choose("frontend-classic"));
    assert_eq!(frontend::load(&file), Some(Frontend::Classic));
    assert!(
        !bare.load(Ordering::Relaxed),
        "the plain frontend has the language bar in the page"
    );
    assert!(app.choose("frontend-a"));
    assert!(bare.load(Ordering::Relaxed));
    assert_eq!(frontend::load(&file), Some(Frontend::Confluence));
    assert!(!app.choose("frontend-x"), "an unknown entry does nothing");
}

#[test]
fn a_run_with_an_override_remembers_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (app, _) = app(Frontend::Notion, None, false);
    assert!(app.choose("frontend-classic"));
    assert!(!frontend::settings_file(dir.path()).exists());
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
    assert_eq!(Frontend::parse("classic"), Some(Frontend::Classic));
    assert_eq!(Frontend::parse("c"), None);
}

#[test]
fn the_help_speaks_of_frontends() {
    let help = fs_wiki_app::args::HELP;
    assert!(help.contains("View > Frontend"));
    // (`--facet` is a different word.)
    let words = help
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase);
    for word in words {
        assert!(
            !["face", "faces", "gesicht"].contains(&word.as_str()),
            "{word}"
        );
    }
}
