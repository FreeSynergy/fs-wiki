//! Export options.
//!
//! Pattern: Value object — plain `Copy` flags passed down to every renderer.

/// Options of the Markdown exporter.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExportOptions {
    /// Also write layer marks with attribute syntax (`{{name}}`, `[text]{#name}`).
    pub layers: bool,
    /// Write callouts as mkdocs admonitions (`!!! note "Title"`, body indented by 4) instead
    /// of `> [!note] Title`. Both are read on import.
    pub admonitions: bool,
}
