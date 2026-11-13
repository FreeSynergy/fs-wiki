//! The modal of the wiki window: a name asked for, or a question answered with a button.
//!
//! Pattern: State + Decorator — the window holds at most one modal in a slot shared with its surface; the surface draws the
//! page and, over it, a dimmed layer with the modal in the middle. The dialogs are the `NameDialog` of `fs-components` and a
//! question with two buttons built the same way; Enter and Escape reach them as broadcast keys. Blinc comes only through
//! `fs_components::blinc`.

use std::sync::{Arc, Mutex, PoisonError};

use fs_components::blinc::cn::components::button::{ButtonBuilder, ButtonVariant};
use fs_components::blinc::layout::div::{Div, div};
use fs_components::blinc::layout::prelude::text;
use fs_components::blinc::layout::widgets::overlay_stack::{
    CloseReason, OverlayBuilder, OverlayHandle,
};
use fs_components_widgets::name_dialog::NameDialog;

/// How the modal is drawn: called at every redraw (the state of the dialog lives outside the elements it draws).
pub type View = Arc<dyn Fn() -> Div + Send + Sync>;

/// What taking a name does (what Enter or OK does with the text in the field).
pub type Submit = Arc<dyn Fn(&str) + Send + Sync>;
/// What a button does.
pub type Act = Arc<dyn Fn() + Send + Sync>;

/// A modal: how it is drawn and what its answers do (the same closures its buttons and keys call, so a test can answer
/// without a click).
#[derive(Clone)]
pub struct Modal {
    /// How it is drawn.
    pub view: View,
    /// A name dialog: what the name does.
    pub submit: Option<Submit>,
    /// A question: what yes does.
    pub yes: Option<Act>,
    /// What no, Cancel and Escape do.
    pub no: Option<Act>,
}

/// The one modal of a window, or none. It is an entry of Blinc's overlay stack (kind modal: a dimmed backdrop, input blocked
/// below, Escape closes) because text of the page below is drawn above any element laid over it; the stack is what draws
/// above everything.
#[derive(Default)]
pub struct ModalSlot {
    current: Mutex<Option<Modal>>,
    handle: Mutex<Option<u64>>,
}

impl ModalSlot {
    /// Show `modal` over the page (a modal that is open goes first).
    pub fn set(self: &Arc<Self>, modal: Modal) {
        self.clear();
        let (view, no) = (Arc::clone(&modal.view), modal.no.clone());
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = Some(modal);
        let me = Arc::clone(self);
        let raw = OverlayBuilder::modal()
            .centered()
            .dismissable_by_escape(true)
            .dismissable_by_click_outside(false)
            .on_close(move |reason| {
                if matches!(reason, CloseReason::Escape) {
                    // The stack closed it already: forget the handle so `clear` does not close it again.
                    me.handle
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .take();
                    if let Some(no) = &no {
                        no();
                    }
                }
            })
            .content(move || view())
            .show()
            .raw();
        *self.handle.lock().unwrap_or_else(PoisonError::into_inner) = Some(raw);
    }

    /// Take the modal away.
    pub fn clear(&self) {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let raw = self
            .handle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(raw) = raw {
            OverlayHandle::from_raw(raw).close();
        }
    }

    fn current(&self) -> Option<Modal> {
        self.current
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Whether a modal is shown.
    pub fn is_open(&self) -> bool {
        self.current().is_some()
    }

    /// Answer a name dialog with `name`, as Enter does; false when no name dialog is shown.
    pub fn submit(&self, name: &str) -> bool {
        self.current()
            .and_then(|m| m.submit)
            .map(|f| f(name))
            .is_some()
    }

    /// Press yes of a question; false when none is shown.
    pub fn yes(&self) -> bool {
        self.current().and_then(|m| m.yes).map(|f| f()).is_some()
    }

    /// Press no, Cancel or Escape; false when no modal is shown.
    pub fn no(&self) -> bool {
        self.current().and_then(|m| m.no).map(|f| f()).is_some()
    }
}

/// The style sheet of the modal: theme variables only.
pub const STYLES: &str = "
.fs-wiki-modal { display: flex; flex-direction: column; width: 420px; gap: var(--space-2); }
.fs-wiki-modal-error { color: var(--error); background: var(--surface); border-width: 1px; border-color: var(--border-error); font-size: var(--text-sm); padding: var(--space-2); border-radius: var(--radius-default); }
.fs-wiki-modal-body { color: var(--text-secondary); }
";

/// The view of a name dialog with an error line under it (empty = none). A blank name on Enter is told here (the dialog
/// itself just stays open).
pub fn name_view(dialog: NameDialog, error: Arc<Mutex<String>>, blank: String) -> View {
    Arc::new(move || {
        let said = error.lock().unwrap_or_else(PoisonError::into_inner).clone();
        let (dialog_keys, blank_error, blank_text) =
            (dialog.clone(), Arc::clone(&error), blank.clone());
        let mut column = div().class("fs-wiki-modal").child(dialog.build());
        if !said.is_empty() {
            column = column.child(text(said).class("fs-wiki-modal-error"));
        }
        column.on_key_down(move |ctx| {
            if ctx.key_code == 13 && dialog_keys.text().trim().is_empty() {
                blank_error
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone_from(&blank_text);
            }
        })
    })
}

/// What a question shows and does.
#[derive(Clone)]
pub struct Question {
    /// The title (the question itself).
    pub title: String,
    /// The sentence under it.
    pub body: String,
    /// The button that does it.
    pub yes_word: String,
    /// The button that leaves things as they are.
    pub no_word: String,
    /// What the yes button (and Enter) does.
    pub on_yes: Arc<dyn Fn() + Send + Sync>,
    /// What the no button (and Escape) does.
    pub on_no: Arc<dyn Fn() + Send + Sync>,
}

/// The modal of a question: title, sentence, the two buttons.
pub fn question_view(q: Question) -> Modal {
    let (answer_yes, answer_no) = (Arc::clone(&q.on_yes), Arc::clone(&q.on_no));
    let view: View = Arc::new(move || {
        let (yes, no) = (Arc::clone(&q.on_yes), Arc::clone(&q.on_no));
        let (keys_yes, keys_no) = (Arc::clone(&q.on_yes), Arc::clone(&q.on_no));
        let button = |tag: &str, word: &str, variant| {
            ButtonBuilder::with_key(format!("fs-wiki-q-{tag}"), word.to_owned()).variant(variant)
        };
        let buttons = div()
            .class("fs-nd-buttons")
            .child(
                button("no", &q.no_word, ButtonVariant::Outline)
                    .on_click(move |_| no())
                    .build_component()
                    .id("fs-wiki-q-no"),
            )
            .child(
                button("yes", &q.yes_word, ButtonVariant::Primary)
                    .on_click(move |_| yes())
                    .build_component()
                    .id("fs-wiki-q-yes"),
            );
        div().class("fs-wiki-modal").child(
            div()
                .id("fs-wiki-question")
                .class("fs-nd")
                .child(text(q.title.clone()).class("fs-nd-title"))
                .child(text(q.body.clone()).class("fs-wiki-modal-body"))
                .child(buttons)
                .on_key_down(move |ctx| match ctx.key_code {
                    13 => keys_yes(),
                    27 => keys_no(),
                    _ => {}
                }),
        )
    });
    Modal {
        view,
        submit: None,
        yes: Some(answer_yes),
        no: Some(answer_no),
    }
}
