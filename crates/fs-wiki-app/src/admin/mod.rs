//! The Admin window of the wiki: the standard shell (ADR 0065) with the generated frontend (ADR 0066) over the
//! type descriptions of the server.
//!
//! Pattern: Facade — `open` asks the server for its descriptions, makes a session per data type with the backend of that
//! type, and builds the window; nothing here draws (no render code in the program, ADR 0066). The backends are
//! Adapters from the frontend's `Backend` to `Texts` (one file per type).

mod ctx;
mod folder;
mod link;
mod page;
mod page_more;
mod records;
mod revision;
mod say;
mod trash;
mod window;

pub use ctx::{Ctx, unsupported};
pub use folder::FolderBackend;
pub use link::LinkBackend;
pub use page::PageBackend;
pub use revision::RevisionBackend;
pub use say::Words;
pub use trash::TrashBackend;
pub use window::{about, backend_for, open, shell, views};
