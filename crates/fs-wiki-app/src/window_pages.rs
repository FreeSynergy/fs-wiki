//! What the menu Page does in the window: ask for a name, rename, delete, discard a draft.
//!
//! Pattern: Command — every entry of the menu Page is one method of the window's `Switcher`; each asks (a name dialog, a
//! question with two buttons), then calls the `PageMaker` (the same storage layer as the pictures use) and tells the
//! result in the status line; a refused name stays in its dialog with the reason under it. Nothing is deleted without a
//! question, and a delete is a move to the trash.

use std::sync::{Arc, PoisonError};

use fs_components_app::Status;
use fs_components_widgets::name_dialog::Purpose;

use crate::frontend_window::{
    PAGE_DELETE, PAGE_DISCARD, PAGE_NEW, PAGE_RENAME, PAGE_SUB, Switcher,
};
use crate::page_words::{create_error, page_words};
use crate::source::{CreateError, PageMaker};
use crate::window_modal::{Question, question_view};

/// What a name asked for in the dialog is for.
#[derive(Clone)]
pub(crate) enum Named {
    New,
    Sub(String),
    Rename(String),
}

/// What the reader shows of the open page: its path, its title, whether it is in the editor.
struct Open {
    path: Option<String>,
    title: String,
    editing: bool,
}

impl Switcher {
    fn open_page(&self) -> Open {
        self.surface.current().with_reader(|r| Open {
            path: r.current_path().map(str::to_owned),
            title: r.page_title(),
            editing: r.editing().is_some(),
        })
    }

    pub(crate) fn problem(&self, text: String) {
        self.app.set_status(Some(Status::Problem(text)));
    }

    fn info(&self, text: String) {
        self.app.set_status(Some(Status::Info(text)));
    }

    /// Draw again (the menus are set again, which redraws everything) after the modal or the pages changed.
    pub(crate) fn redraw(self: &Arc<Self>) {
        if let Err(e) = self.install(self.shown()) {
            eprintln!("fs-wiki: the menus are not set ({e})");
        }
    }

    pub(crate) fn close_modal(self: &Arc<Self>) {
        self.surface.modal.clear();
        self.redraw();
    }

    /// An entry of the menu Page was chosen.
    pub(crate) fn page_action(self: &Arc<Self>, id: &str) {
        let Some(maker) = self.setup.pages.clone() else {
            return;
        };
        let words = page_words(self.setup.german);
        let open = self.open_page();
        if !maker.can_create() {
            return self.problem(create_error(&CreateError::ReadOnly, self.setup.german));
        }
        let discard = id == PAGE_DISCARD;
        if open.editing && !discard {
            return self.problem(words.finish_edit);
        }
        if !discard && id != PAGE_NEW && open.path.is_none() {
            return self.problem(words.need_page);
        }
        match (id, open.path.clone()) {
            (PAGE_NEW, _) => self.ask_name(Purpose::NewPage, "", "", Named::New),
            (PAGE_SUB, Some(p)) => self.ask_name(Purpose::SubPage, &open.title, "", Named::Sub(p)),
            (PAGE_RENAME, Some(p)) => {
                self.ask_name(Purpose::Rename, &open.title, &open.title, Named::Rename(p));
            }
            (PAGE_DELETE, Some(p)) => self.ask_delete(&maker, &p, &open.title),
            (PAGE_DISCARD, Some(p)) if open.editing => self.ask_discard(&maker, &p, &open),
            _ => self.problem(words.no_draft),
        }
    }

    /// The name was taken in the dialog: do it; on success the dialog goes, on a refusal it stays (the caller says why).
    pub(crate) fn apply_name(
        self: &Arc<Self>,
        named: &Named,
        name: &str,
    ) -> Result<(), CreateError> {
        let maker = self.setup.pages.clone().ok_or(CreateError::ReadOnly)?;
        let widget = self.surface.current();
        let texts = self.texts();
        let (path, told) = match named {
            Named::New => {
                let path = maker.create_exact(None, name)?;
                (path.clone(), texts.say("msg-created", &[("path", &path)]))
            }
            Named::Sub(parent) => {
                let path = maker.create_exact(Some(parent), name)?;
                (path.clone(), texts.say("msg-created", &[("path", &path)]))
            }
            Named::Rename(old) => {
                let path = match maker.rename_page(old, name) {
                    Ok(path) => path,
                    // The page moved but its title was not written, or only part of it was done: the dialog does not stay
                    // on the old path; it goes, the page tree is read again and the person is told how it stands.
                    Err(e @ (CreateError::Partial { .. } | CreateError::Half(_))) => {
                        let at = match &e {
                            CreateError::Partial { path, .. } => Some(path.clone()),
                            _ => None,
                        };
                        widget.with_reader(|r| {
                            r.refresh_entries();
                            if let Some(at) = &at {
                                r.open(at);
                            }
                        });
                        self.relayout(self.shown());
                        self.surface.modal.clear();
                        self.problem(create_error(&e, self.setup.german));
                        self.redraw();
                        return Ok(());
                    }
                    Err(e) => return Err(e),
                };
                (path.clone(), texts.say("msg-renamed", &[("path", &path)]))
            }
        };
        let renamed = matches!(named, Named::Rename(_));
        if !renamed && let Some(revision) = maker.revision(&path) {
            self.fresh
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(path.clone(), revision);
        }
        widget.with_reader(|r| {
            r.refresh_entries();
            r.open(&path);
            if !renamed {
                r.begin_edit();
            }
        });
        self.relayout(self.shown());
        self.surface.modal.clear();
        self.info(told);
        self.redraw();
        Ok(())
    }

    fn ask_delete(self: &Arc<Self>, maker: &Arc<dyn PageMaker>, path: &str, title: &str) {
        let t = self.texts();
        let subs = maker.subpage_count(path);
        let body = if subs == 0 {
            t.plain("dlg-delete-body")
        } else {
            t.say_count("dlg-delete-body-sub", subs as u64 + 1, &[])
        };
        let (yes, no, path) = (Arc::clone(self), Arc::clone(self), path.to_owned());
        let title_told = title.to_owned();
        self.surface.modal.set(question_view(Question {
            title: t.say("dlg-delete-title", &[("title", title)]),
            body,
            yes_word: t.plain("dlg-delete-yes"),
            no_word: t.plain("dlg-keep"),
            on_yes: Arc::new(move || yes.delete(&path, &title_told)),
            on_no: Arc::new(move || no.close_modal()),
        }));
        self.redraw();
    }

    fn delete(self: &Arc<Self>, path: &str, title: &str) {
        self.surface.modal.clear();
        let Some(maker) = self.setup.pages.clone() else {
            return;
        };
        match maker.trash_page(path) {
            Ok(_) => {
                self.leave_gone_page();
                let told = self.texts().say("msg-deleted", &[("title", title)]);
                self.info(told);
            }
            Err(e) => self.problem(create_error(&e, self.setup.german)),
        }
        self.redraw();
    }

    /// The page shown is gone: the list is read again and the first page opens.
    fn leave_gone_page(self: &Arc<Self>) {
        self.surface.current().with_reader(|r| {
            r.refresh_entries();
            if let Some(first) = r.entries().first().map(|e| e.path.clone()) {
                r.open(&first);
            }
        });
        self.relayout(self.shown());
    }

    fn ask_discard(self: &Arc<Self>, maker: &Arc<dyn PageMaker>, path: &str, open: &Open) {
        let t = self.texts();
        let fresh = self.is_fresh(maker.as_ref(), path);
        let (yes, no, path) = (Arc::clone(self), Arc::clone(self), path.to_owned());
        self.surface.modal.set(question_view(Question {
            title: t.say("dlg-discard-title", &[("title", &open.title)]),
            body: t.plain(if fresh {
                "dlg-discard-new-body"
            } else {
                "dlg-discard-body"
            }),
            yes_word: t.plain("dlg-discard-yes"),
            no_word: t.plain("dlg-keep"),
            on_yes: Arc::new(move || yes.discard(&path, fresh)),
            on_no: Arc::new(move || no.close_modal()),
        }));
        self.redraw();
    }

    /// A page made in this window that was not saved since (its text is still what the window wrote).
    fn is_fresh(&self, maker: &dyn PageMaker, path: &str) -> bool {
        let fresh = self.fresh.lock().unwrap_or_else(PoisonError::into_inner);
        fresh
            .get(path)
            .is_some_and(|made| maker.revision(path).as_ref() == Some(made))
    }

    fn discard(self: &Arc<Self>, path: &str, fresh: bool) {
        self.surface.modal.clear();
        // The first call asks (when there are changes), the second discards.
        self.surface.current().with_reader(|r| {
            r.cancel_edit();
            r.cancel_edit();
        });
        let t = self.texts();
        if fresh && let Some(maker) = self.setup.pages.clone() {
            self.fresh
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(path);
            match maker.trash_page(path) {
                Ok(_) => {
                    self.leave_gone_page();
                    self.info(t.plain("msg-discarded-new"));
                }
                Err(e) => self.problem(create_error(&e, self.setup.german)),
            }
        } else {
            self.info(t.plain("msg-discarded"));
        }
        self.redraw();
    }
}
