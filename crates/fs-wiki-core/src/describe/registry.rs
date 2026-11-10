//! The registry of the wiki's data types.
//!
//! Pattern: Registry (`coding.md` section 2): every type enters through `DescribesType`.

use fs_types::{DescriptionProblem, TypeDescription, TypeRegistry};

use crate::{LinkEntry, Node, RevisionInfo, TextEntry, TrashItem};

/// The registry with every data type of the wiki: `Page`, `Folder`, `Revision`, `TrashEntry`, `Link`.
///
/// # Errors
/// A defect of a description (a bug of this crate; the tests keep it from happening).
pub fn registry() -> Result<TypeRegistry, DescriptionProblem> {
    let mut registry = TypeRegistry::new();
    registry
        .register::<TextEntry>()?
        .register::<Node>()?
        .register::<RevisionInfo>()?
        .register::<TrashItem>()?
        .register::<LinkEntry>()?;
    Ok(registry)
}

/// The descriptions of the wiki in name order, for a program that serves them over `Texts.types`.
///
/// # Errors
/// As [`registry`].
pub fn descriptions() -> Result<Vec<TypeDescription>, DescriptionProblem> {
    Ok(registry()?.iter().cloned().collect())
}

/// Name of the owner program (ADR 0050).
pub const PROGRAM: &str = "fs-wiki";

/// Names of the commands that `requirements.md` asks for and the core does not have (the table of what is supported
/// starts from the complement of this list).
#[must_use]
pub fn wished_commands() -> Vec<(&'static str, &'static str)> {
    super::page_wish::NAMES
        .iter()
        .map(|n| ("Page", *n))
        .chain(super::link::WISHED.iter().map(|n| ("Link", *n)))
        .collect()
}
