//! Templates (ADR 0042 §9): `{{name}}` and `{{date:FORMAT}}` placeholders, plus variable
//! point marks (ADR 0033) in block documents.
//!
//! Pattern: Template method — find placeholders, resolve each against [`Values`], splice the
//! result into the text keeping marks. `\{{` writes a literal `{{`. Date formats use a
//! strftime subset: `%Y %y %m %d %e %H %M %S %B %b %A %a %j %%`.

mod datetime;
mod error;
mod format;
mod instantiate;
mod placeholder;
mod values;

pub use datetime::DateTime;
pub use error::TemplateError;
pub use instantiate::{instantiate, instantiate_str};
pub use placeholder::{find_placeholders, variables, Placeholder};
pub use values::Values;
