//! What every backend of the Admin frontend shares: the texts, the runtime, the reading of input and the errors.
//!
//! Pattern: Adapter — the synchronous `Backend` of `fs-admin-frontend` over the asynchronous `Texts` of the core
//! (`Handle::block_on`; the window thread is not a task of that runtime). Errors of the core become `BackendError`:
//! the frontend's vocabulary for a refusal is the findings of `fs-types`, which have no word for "path taken", so a
//! refusal of the program is told as `Unavailable` with the English text for the log and the person sees a code.

use std::sync::Arc;

use fs_admin_frontend::{BackendError, Values};
use fs_objects::ObjectId;
use fs_wiki_core::{Error, Texts};
use tokio::runtime::Handle;

/// How many records a list reads at most.
pub const LIST_LIMIT: usize = 500;

/// The unsupported answer for a command that is described but not carried out (`DESIGN.md`, "Unsupported").
pub fn unsupported(name: &str) -> BackendError {
    BackendError::Unavailable(format!("unsupported:{name}"))
}

/// The wiki and the runtime to ask it on.
#[derive(Clone)]
pub struct Ctx {
    /// The texts (the running server, through the client).
    pub texts: Arc<dyn Texts>,
    handle: Handle,
}

impl Ctx {
    /// Texts and the runtime they are asked on.
    pub fn new(texts: Arc<dyn Texts>, handle: Handle) -> Self {
        Self { texts, handle }
    }

    /// Wait for a call of the core.
    ///
    /// # Errors
    /// The error of the core as the frontend's ([`fail`]).
    pub fn block<T>(
        &self,
        f: impl std::future::Future<Output = fs_wiki_core::Result<T>>,
    ) -> Result<T, BackendError> {
        self.handle.block_on(f).map_err(fail)
    }
}

/// The error of the core as the frontend's.
pub fn fail(e: Error) -> BackendError {
    match e {
        Error::NotFound(_)
        | Error::NoSuchNode(_)
        | Error::NoSuchRevision { .. }
        | Error::NoSuchFolder(_) => BackendError::NotFound,
        Error::Unsupported(u) => BackendError::Unavailable(format!("unsupported:{u}")),
        other => BackendError::Unavailable(other.to_string()),
    }
}

/// The first value of a field, trimmed; `None` when missing or empty.
pub fn one<'a>(input: &'a Values, field: &str) -> Option<&'a str> {
    input
        .get(field)
        .and_then(|v| v.first())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

/// Whether the field is in the input at all (an empty list means "cleared").
pub fn sent(input: &Values, field: &str) -> bool {
    input.contains_key(field)
}

/// A required field; a missing one is a finding of the form.
///
/// # Errors
/// `Rejected` with a `Missing` finding.
pub fn need<'a>(input: &'a Values, field: &str) -> Result<&'a str, BackendError> {
    one(input, field).ok_or_else(|| {
        BackendError::Rejected(vec![fs_types_admin::RecordIssue {
            field: field.to_owned(),
            index: 0,
            problem: fs_types_admin::RecordProblem::Missing,
        }])
    })
}

/// A required id.
///
/// # Errors
/// `Rejected` when missing, `NotFound` when it is not an id of ours.
pub fn id(input: &Values, field: &str) -> Result<ObjectId, BackendError> {
    need(input, field)?
        .parse()
        .map_err(|_| BackendError::NotFound)
}

/// An optional id.
///
/// # Errors
/// `NotFound` when given and not an id.
pub fn optional_id(input: &Values, field: &str) -> Result<Option<ObjectId>, BackendError> {
    one(input, field)
        .map(|s| s.parse().map_err(|_| BackendError::NotFound))
        .transpose()
}

/// A text value as a one-element list; nothing for an empty text.
pub fn text(s: &str) -> Vec<String> {
    if s.is_empty() {
        Vec::new()
    } else {
        vec![s.to_owned()]
    }
}

/// A value that should be a number and is not.
pub fn not_a_number(field: &str) -> BackendError {
    BackendError::Rejected(vec![fs_types_admin::RecordIssue {
        field: field.to_owned(),
        index: 0,
        problem: fs_types_admin::RecordProblem::Value(fs_types_admin::TypeProblem::Character),
    }])
}

/// A placement word of the description.
///
/// # Errors
/// `Rejected` when the word is none of first, last, before, after.
pub fn placement(word: &str) -> Result<fs_wiki_core::Placement, BackendError> {
    use fs_wiki_core::Placement as P;
    Ok(match word {
        "first" => P::First,
        "last" => P::Last,
        "before" => P::Before,
        "after" => P::After,
        _ => return Err(not_a_number("placement")),
    })
}

/// A date-time field as milliseconds since the epoch; 0 when not given (no restriction).
///
/// # Errors
/// `Rejected` when it is not an RFC 3339 date-time.
pub fn millis(input: &Values, field: &str) -> Result<i64, BackendError> {
    match one(input, field) {
        None => Ok(0),
        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|t| t.timestamp_millis())
            .map_err(|_| not_a_number(field)),
    }
}
