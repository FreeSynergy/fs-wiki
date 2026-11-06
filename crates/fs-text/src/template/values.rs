//! Values for instantiation.
//!
//! Pattern: Builder-ish value bag — `Values::new(now).set(k, v)…`.

use super::datetime::DateTime;
use std::collections::BTreeMap;

/// Values for instantiation. Entered values go into the database *and* the document.
#[derive(Clone, Debug)]
pub struct Values {
    /// Variable values by name.
    pub vars: BTreeMap<String, String>,
    /// Time used by `{{date:…}}`.
    pub now: DateTime,
    /// Leave unknown placeholders in place instead of failing.
    pub lenient: bool,
}

impl Values {
    /// Empty values at time `now`.
    #[must_use]
    pub fn new(now: DateTime) -> Self {
        Self {
            vars: BTreeMap::new(),
            now,
            lenient: false,
        }
    }
    /// Set a variable.
    #[must_use]
    pub fn set(mut self, k: &str, v: &str) -> Self {
        self.vars.insert(k.to_string(), v.to_string());
        self
    }
}
