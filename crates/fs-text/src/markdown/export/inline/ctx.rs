//! Rendering context.
//!
//! Pattern: Value object — where the inline text sits (table cell, heading) and the options.

use crate::markdown::ExportOptions;

/// Where inline text is rendered; decides escaping.
#[derive(Clone, Copy)]
pub(in crate::markdown::export) struct Ctx {
    pub table: bool,
    pub heading: bool,
    pub opts: ExportOptions,
}
