//! Template errors.
//!
//! Pattern: Error enum (thiserror).

/// Why a template could not be filled.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TemplateError {
    /// No value for a variable.
    #[error("no value for variable `{0}`")]
    Missing(String),
    /// Unknown `%x` directive.
    #[error("unknown date format directive `%{0}`")]
    BadFormat(char),
}
