//! Small builders the descriptions share: label keys from names, references, commands.
//!
//! Pattern: Builder helpers (`coding.md` section 2; they only call the Builder chain of `fs-types`).

use fs_types::{BasicType, CommandDescription, CommandKind, FieldDescription};

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
    FieldDescription::new(name, BasicType::Reference)
        .target(target)
        .required()
        .label(label)
}

/// A reference to a record of the data type `target`, optional.
pub(crate) fn optional_reference(name: &str, target: &str, label: &str) -> FieldDescription {
    FieldDescription::new(name, BasicType::Reference)
        .target(target)
        .label(label)
}

/// A command with a label key `wiki.<ty>.<name>`.
pub(crate) fn command(ty: &str, name: &str, kind: CommandKind) -> CommandDescription {
    CommandDescription::new(name, kind, &format!("wiki.{ty}.command.{}", kebab(name)))
}

/// A field with a label key `wiki.<ty>.<name>`.
pub(crate) fn field(ty: &str, name: &str, basic: BasicType) -> FieldDescription {
    FieldDescription::new(name, basic).label(&format!("wiki.{ty}.{}", kebab(name)))
}
