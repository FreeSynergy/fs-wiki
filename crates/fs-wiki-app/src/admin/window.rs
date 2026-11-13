//! Sessions and window from the descriptions.
//!
//! Pattern: Factory — a backend per data type name (`backend_for`), a session and a surface per description (`views`),
//! the window around them (`open`).

use std::sync::Arc;

use fs_admin_frontend::view::{AboutInfo, AdminSurface};
use fs_admin_frontend::window::{TypeView, admin_window};
use fs_admin_frontend::{Backend, Session, SystemClock};
use fs_components_app::{AppError, AppWidget, WindowOptions, launch};
use fs_components_fields::{Look, Say};
use fs_components_styles::StyleId;
use fs_types::TypeRegistry;

use super::{Ctx, FolderBackend, LinkBackend, PageBackend, RevisionBackend, TrashBackend, Words};

/// The backend of a data type of the wiki; none for a type this version does not know.
pub fn backend_for(name: &str, ctx: &Ctx) -> Option<Arc<dyn Backend>> {
    Some(match name {
        "Page" => Arc::new(PageBackend::new(ctx.clone())),
        "Folder" => Arc::new(FolderBackend(ctx.clone())),
        "Revision" => Arc::new(RevisionBackend(ctx.clone())),
        "TrashEntry" => Arc::new(TrashBackend(ctx.clone())),
        "Link" => Arc::new(LinkBackend(ctx.clone())),
        _ => return None,
    })
}

/// One view per described type that has a backend, in the order of the navigation: pages first.
pub fn views(registry: &TypeRegistry, ctx: &Ctx) -> Vec<TypeView> {
    views_in(registry, ctx, false)
}

/// [`views`] with the words of the interface language.
pub fn views_in(registry: &TypeRegistry, ctx: &Ctx, german: bool) -> Vec<TypeView> {
    let say: Arc<dyn Say> = Arc::new(Words::new(german));
    ["Page", "Folder", "Revision", "TrashEntry", "Link"]
        .into_iter()
        .filter_map(|name| {
            let description = registry.get(name)?;
            let backend = backend_for(name, ctx)?;
            let session = Session::new(description.clone(), backend, Arc::new(SystemClock::new()));
            let surface = AdminSurface::new(
                session,
                Look::new(StyleId::Admin, false),
                Arc::clone(&say),
                None,
            );
            let label = format!("type.{}", name.trim_end_matches("Entry").to_lowercase());
            Some(TypeView {
                id: name.to_owned(),
                label,
                surface: Arc::new(surface),
            })
        })
        .collect()
}

/// The About page: name, version, origin, licence, repository.
pub fn about(origin: &str) -> AboutInfo {
    AboutInfo {
        name: "FreeSynergy.Wiki".into(),
        version: format!("v{}", env!("CARGO_PKG_VERSION")),
        origin: origin.into(),
        licence: "MIT OR Apache-2.0".into(),
        repository: "https://github.com/FreeSynergy/fs-wiki".into(),
    }
}

/// The window, not yet opened.
///
/// # Errors
/// The shell cannot be built (two views of one id).
pub fn shell(registry: &TypeRegistry, ctx: &Ctx, origin: &str) -> Result<AppWidget, AppError> {
    shell_in(registry, ctx, origin, false)
}

/// [`shell`] with the words of the interface language.
///
/// # Errors
/// The shell cannot be built (two views of one id).
pub fn shell_in(
    registry: &TypeRegistry,
    ctx: &Ctx,
    origin: &str,
    german: bool,
) -> Result<AppWidget, AppError> {
    let say: Arc<dyn Say> = Arc::new(Words::new(german));
    admin_window(
        "FreeSynergy Wiki",
        &say,
        views_in(registry, ctx, german),
        about(origin),
        "https://github.com/FreeSynergy/fs-wiki".into(),
    )
}

/// Open the window and run until it is closed.
///
/// # Errors
/// The window or the graphics cannot be started.
pub fn open(registry: &TypeRegistry, ctx: &Ctx, origin: &str, german: bool) -> Result<(), AppError> {
    launch(WindowOptions::default(), shell_in(registry, ctx, origin, german)?)
}
