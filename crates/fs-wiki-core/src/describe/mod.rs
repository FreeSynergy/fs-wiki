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

mod build;
mod folder;
mod link;
mod page;
mod page_wish;
mod registry;
mod revision;
mod support;
mod trash;

pub(crate) use build::{command, field, optional_reference, reference};
pub use registry::{descriptions, registry, wished_commands, PROGRAM};
pub use support::{is_supported, unsupported, SUPPORTED};
