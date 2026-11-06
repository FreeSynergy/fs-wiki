//! Interchange formats: the interfaces every file format implements.
//!
//! Pattern: Adapter (interfaces) — Markdown is the first adapter ([`crate::markdown`]);
//! others (HTML, org, plain text) implement the same two traits.

mod exporter;
mod importer;

pub use exporter::Exporter;
pub use importer::Importer;
