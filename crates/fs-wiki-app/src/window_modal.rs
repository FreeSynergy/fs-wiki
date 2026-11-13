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

/// The one modal of a window, or none.
#[derive(Default)]
pub struct ModalSlot(Mutex<Option<Modal>>);

impl ModalSlot {
    /// Show `modal` over the page.
    pub fn set(&self, modal: Modal) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(modal);
    }

    fn current(&self) -> Option<Modal> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Answer a name dialog with `name`, as Enter does; false when no name dialog is shown.
    pub fn submit(&self, name: &str) -> bool {
        self.current().and_then(|m| m.submit).map(|f| f(name)).is_some()
    }

    /// Press yes of a question; false when none is shown.
    pub fn yes(&self) -> bool {
        self.current().and_then(|m| m.yes).map(|f| f()).is_some()
    }

    /// Press no, Cancel or Escape; false when no modal is shown.
    pub fn no(&self) -> bool {
        self.current().and_then(|m| m.no).map(|f| f()).is_some()
    }

    /// Take the modal away.
    pub fn clear(&self) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }

    /// Whether a modal is shown.
    pub fn is_open(&self) -> bool {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).is_some()
    }

    /// The page with the modal over it (the page alone when there is none), `width` x `height`.
    pub fn over(&self, page: impl fs_components_widgets::Element + 'static, width: f32, height: f32) -> Div {
        let view = self.current().map(|m| m.view);
        let root = div().w(width).h(height).relative().child(page);
        match view {
            None => root,
            Some(view) => root.child(
                div()
                    .class("fs-wiki-scrim")
                    .absolute()
                    .left(0.0)
                    .top(0.0)
                    .w(width)
                    .h(height)
                    .child(view()),
            ),
        }
    }
}

/// The style sheet of the modal layer: theme variables only.
pub const STYLES: &str = "
.fs-wiki-scrim { display: flex; flex-direction: column; align-items: center; justify-content: center; background: var(--surface-overlay); }
.fs-wiki-modal { display: flex; flex-direction: column; width: 100%; max-width: 420px; gap: var(--space-2); }
.fs-wiki-modal-error { color: var(--error); font-size: var(--text-sm); padding-left: var(--space-1); }
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
                *blank_error.lock().unwrap_or_else(PoisonError::into_inner) = blank_text.clone();
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
