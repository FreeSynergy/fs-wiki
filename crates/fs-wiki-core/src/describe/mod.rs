//! The data types of the wiki program, described for a generated frontend (ADR 0066).
//!
//! Pattern: Registry (`coding.md` section 2): each data type enters the `TypeRegistry` of `fs-types` through the trait
//! `DescribesType` (the trait is the interface, ADR 0043); a description itself is a Value object built with the
//! Builder chain of `fs-types`. The standard of each type is chosen where its description is (module head), the reason
//! in `DESIGN.md` ("Types").
//!
//! Every command of the core is in a description, and so is every command of `requirements.md`; whether the core can
//! carry one out is not said here (types.capnp has no `supported` field yet): the program that serves the frontend
//! keeps a table for that, `DESIGN.md` section "Unsupported".

mod folder;
mod link;
mod page;
mod page_wish;
mod revision;
mod support;
mod trash;

pub use support::{is_supported, unsupported, SUPPORTED};

use fs_types::{BasicType, CommandDescription, CommandKind, DescriptionProblem, FieldDescription, TypeDescription, TypeRegistry};

use crate::{LinkEntry, Node, RevisionInfo, TextEntry, TrashItem};

/// Name of the owner program (ADR 0050).
pub const PROGRAM: &str = "fs-wiki";

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

/// `redirectTo` as `redirect-to`: a translation key is lower case.
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// A reference to a record of the data type `target`, required, named `name`.
pub(crate) fn reference(name: &str, target: &str, label: &str) -> FieldDescription {
    FieldDescription::new(name, BasicType::Reference).target(target).required().label(label)
}

/// A reference to a record of the data type `target`, optional.
pub(crate) fn optional_reference(name: &str, target: &str, label: &str) -> FieldDescription {
    FieldDescription::new(name, BasicType::Reference).target(target).label(label)
}

/// A command with a label key `wiki.<ty>.<name>`.
pub(crate) fn command(ty: &str, name: &str, kind: CommandKind) -> CommandDescription {
    CommandDescription::new(name, kind, &format!("wiki.{ty}.command.{}", kebab(name)))
}

/// A field with a label key `wiki.<ty>.<name>`.
pub(crate) fn field(ty: &str, name: &str, basic: BasicType) -> FieldDescription {
    FieldDescription::new(name, basic).label(&format!("wiki.{ty}.{}", kebab(name)))
}

/// Names of the commands of a type that `requirements.md` asks for and the core does not have (the program's table
/// of what is supported starts from the complement of this list).
#[must_use]
pub fn wished_commands() -> Vec<(&'static str, &'static str)> {
    page_wish::NAMES.iter().map(|n| ("Page", *n)).chain(link::WISHED.iter().map(|n| ("Link", *n))).collect()
}
