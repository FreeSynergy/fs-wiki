//! The hash of a file or a text.
//!
//! Pattern: none — one function; SHA-256 in hex, so "unchanged" means the same bytes.

use sha2::{Digest, Sha256};
use std::fmt::Write;

/// SHA-256 of `bytes`, in lower-case hex.
pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}
