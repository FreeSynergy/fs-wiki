//! Text storage: trait, rope implementation and the [`Text`] facade.
//!
//! Pattern: Strategy (storage behind [`TextStorage`]) — the rope is one strategy; a CRDT
//! sequence (e.g. `loro`) can replace it by changing [`DefaultStorage`].

mod buffer;
mod rope;
mod storage;

pub use buffer::Text;
pub use rope::RopeStorage;
pub use storage::TextStorage;

/// The storage strategy used by [`Text`] unless another one is named.
pub type DefaultStorage = RopeStorage;
