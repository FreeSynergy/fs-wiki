//! The descriptions the server sent, in the form the Admin frontend reads.
//!
//! Pattern: Adapter — `fs-types` of the server's contract version (v0.0.7) to `fs-types` v0.0.2, the version
//! `fs-admin-frontend` at `4688cee` is built on. They are two crates with equal shapes and different types, so the
//! values are copied field by field; the kinds travel as their wire numbers (a number this side does not know is
//! `unknown`, which the schema promises to read as plain text). Goes away when the frontend moves to the later tag.

use fs_types::{CommandDescription as New, FieldDescription as NewField, TypeDescription as NewType};
use fs_types_admin::{BasicType, CommandDescription, CommandKind, FieldDescription, TypeDescription};

fn field(f: &NewField) -> FieldDescription {
    let basic = BasicType::try_from(f.basic as u16).unwrap_or(BasicType::Unknown);
    let mut out = FieldDescription::new(&f.name, basic).label(&f.label);
    out.required = f.required;
    out.multiple = f.multiple;
    out.target.clone_from(&f.target);
    out.choices.clone_from(&f.choices);
    out
}

fn command(c: &New) -> CommandDescription {
    let kind = CommandKind::try_from(c.kind as u16).unwrap_or(CommandKind::Unknown);
    let mut out = CommandDescription::new(&c.name, kind, &c.label);
    out.input = c.input.iter().map(field).collect();
    out.result = c.result.iter().map(field).collect();
    out
}

/// One description for the frontend.
pub fn description(t: &NewType) -> TypeDescription {
    let mut out = TypeDescription::new(&t.name, &t.program, &t.standard, &t.label);
    out.fields = t.fields.iter().map(field).collect();
    out.commands = t.commands.iter().map(command).collect();
    out
}
