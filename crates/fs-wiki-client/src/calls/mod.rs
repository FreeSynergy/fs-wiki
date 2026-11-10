//! The calls of the contract, one function each, on the connection's thread.
//!
//! Pattern: Facade — this file only declares modules.

pub(crate) mod pages;
pub(crate) mod reads;
pub(crate) mod writes;
pub(crate) mod v2;
pub(crate) mod v2_wire;
