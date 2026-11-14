//! The name dialog of the menu Page: what it asks, what a name does, what stays when a name is refused.
//!
//! Pattern: Command — one method of the window's `Switcher` (the rest of the menu Page is in `window_pages.rs`).

use std::sync::{Arc, Mutex, PoisonError};

use fs_components_primitives::{FieldSize, SearchField};
use fs_components_widgets::name_dialog::{Labels, NameDialog, Purpose};
use fs_components_words::Localized;

use crate::frontend_window::Switcher;
use crate::page_words::create_error_named;
use crate::source::CreateError;
use crate::window_modal::{Modal, NameCard, Submit, name_view};
use crate::window_pages::Named;

impl Switcher {
    pub(crate) fn ask_name(
        self: &Arc<Self>,
        purpose: Purpose,
        about: &str,
        initial: &str,
        named: Named,
    ) {
        let words = Labels::for_locale(self.texts().locale());
        let title = NameDialog::new(&words, purpose, about).title().to_owned();
        let field = SearchField::new(&words.placeholder).size(FieldSize::Body);
        field.set_text(initial);
        let error = Arc::new(Mutex::new(String::new()));
        let (me, fail, typed) = (Arc::clone(self), Arc::clone(&error), field.clone());
        let submit: Submit = Arc::new(move |name| {
            let name = name.trim();
            let result = if name.is_empty() {
                Err(CreateError::EmptyTitle)
            } else {
                me.apply_name(&named, name)
            };
            if let Err(e) = result {
                // A refused name stays in the field, selected: the person types on or changes a letter.
                typed.set_text(name);
                *fail.lock().unwrap_or_else(PoisonError::into_inner) =
                    create_error_named(&e, me.setup.german, name);
                me.redraw();
                typed.focus();
            }
        });
        let (on_ok, on_cancel, cancel) = (Arc::clone(&submit), Arc::clone(self), Arc::clone(self));
        let asked = field.clone();
        let shown = field.clone();
        self.surface.modal.set(Modal {
            view: name_view(NameCard {
                title,
                field: field.clone(),
                error,
                ok_word: words.ok.clone(),
                cancel_word: words.cancel.clone(),
                on_ok: Arc::new(move || on_ok(&asked.text())),
                on_cancel: Arc::new(move || on_cancel.close_modal()),
            }),
            submit: Some(submit),
            yes: None,
            no: Some(Arc::new(move || cancel.close_modal())),
            field_text: Some(Arc::new(move || shown.text())),
        });
        self.redraw();
        field.focus();
    }
}
