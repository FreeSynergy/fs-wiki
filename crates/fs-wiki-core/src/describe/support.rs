//! Which commands the wiki answers today.
//!
//! Pattern: Specification (`coding.md` section 2): a fixed table (type, command) of what the program carries out end to
//! end; everything else in the description is unsupported. The frontend unlocks a control only when the description and the
//! backend (`Backend::supported`, which asks this table) both say so, so it is never changed to switch a control on. When
//! the program gets a command, one row is added here (`// SWITCH`).

use super::registry;

/// The (type, command) pairs that are carried out. Anything else described is unsupported. // SWITCH
pub const SUPPORTED: &[(&str, &str)] = &[
    ("Page", "create"),
    ("Page", "import"),
    ("Page", "createFromTemplate"),
    ("Page", "change"),
    ("Page", "move"),
    ("Page", "setTemplate"),
    ("Page", "setRedirect"),
    ("Page", "reorder"),
    ("Page", "trash"),
    ("Page", "delete"),
    ("Page", "find"),
    ("Page", "search"),
    ("Page", "resolve"),
    ("Page", "export"),
    ("Folder", "find"),
    ("Folder", "create"),
    ("Folder", "rename"),
    ("Folder", "move"),
    ("Folder", "reorder"),
    ("Folder", "delete"),
    ("Revision", "find"),
    ("Revision", "get"),
    ("Revision", "restore"),
    ("Revision", "purge"),
    ("TrashEntry", "find"),
    ("TrashEntry", "restore"),
    ("TrashEntry", "purge"),
    ("Link", "find"),
];

/// Is `command` of `data_type` carried out?
#[must_use]
pub fn is_supported(data_type: &str, command: &str) -> bool {
    SUPPORTED.contains(&(data_type, command))
}

/// Every described command that is not carried out, as (type, command).
///
/// # Panics
/// Never for the shipped descriptions (the tests check that they are valid).
#[must_use]
pub fn unsupported() -> Vec<(String, String)> {
    registry()
        .expect("the descriptions are valid")
        .iter()
        .flat_map(|t| {
            t.commands
                .iter()
                .filter(|c| !is_supported(&t.name, &c.name))
                .map(|c| (t.name.clone(), c.name.clone()))
        })
        .collect()
}
