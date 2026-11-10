//! The words of the Admin frontend for this program, until translation exists (ADR 0053).
//!
//! Pattern: Strategy behind `Say` — a translation key goes in, English comes out. The words of the frontend itself are in
//! `FIXED`; for the keys of this program's descriptions (`wiki.<type>.<part>`, `wiki.<type>.command.<name>`) the last
//! part is spelled out (`redirect-to` becomes "Redirect to"), because the descriptions only ever carry such keys. An
//! unknown key comes back as itself, so a missing word is seen, not hidden.

use fs_components_fields::Say;

/// Words of the frontend itself and of the choices.
const FIXED: &[(&str, &str)] = &[
    ("menu.program", "Program"),
    ("menu.view", "View"),
    ("menu.help", "Help"),
    ("menu.quit", "Quit"),
    ("menu.about", "About"),
    ("action.add", "Add"),
    ("action.back", "Back"),
    ("action.cancel", "Cancel"),
    ("action.close", "Close"),
    ("action.remove", "Remove"),
    ("action.save", "Save"),
    ("message.done", "Done."),
    (
        "message.rejected",
        "The wiki refused this; check the marked fields.",
    ),
    (
        "message.not-found",
        "There is no such page, folder or command.",
    ),
    (
        "message.unavailable",
        "The wiki cannot do this now (see the log), or the command is still to come.",
    ),
    ("message.unsaved", "There is text that is not sent yet."),
    ("list.empty", "Nothing here."),
    ("list.stale", "The list may be old."),
    ("detail.not-loaded", "Not loaded."),
    ("confirm.sure", "Are you sure?"),
    ("value.none", "none"),
    ("about.version", "Version"),
    ("about.origin", "Origin"),
    ("about.licence", "Licence"),
    ("about.repository", "Repository"),
    ("about.unoffered", "Not offered"),
    ("type.page", "Pages"),
    ("type.folder", "Folders"),
    ("type.revision", "History"),
    ("type.trash", "Trash"),
    ("type.link", "Links"),
];

fn spelled(part: &str) -> String {
    let mut s = part.replace('-', " ");
    if let Some(first) = s.get(..1) {
        s = first.to_uppercase() + &s[1..];
    }
    s
}

/// The words, English.
#[derive(Clone, Copy, Debug, Default)]
pub struct Words;

impl Say for Words {
    fn say(&self, key: &str) -> String {
        if let Some((_, text)) = FIXED.iter().find(|(k, _)| *k == key) {
            return (*text).to_owned();
        }
        match key.strip_prefix("wiki.") {
            Some(rest) => rest
                .rsplit('.')
                .next()
                .map_or_else(|| key.to_owned(), spelled),
            None => key.to_owned(),
        }
    }
}
