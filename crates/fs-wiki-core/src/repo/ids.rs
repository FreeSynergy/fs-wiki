//! Ids derived from other ids.
//!
//! Pattern: none (pure functions) — a revision, an order list and the like are found by `get` on an
//! id computed from what they belong to, so no query (which costs the store a scan) is needed. The
//! id is a name-based UUID (SHA-1) with the version bits of a v7 id, which is all `ObjectId` accepts.

use fs_objects::ObjectId;
use uuid::{Builder, Uuid, Variant, Version};

const NAMESPACE: Uuid = Uuid::from_u128(0x6b2f_0a41_95c7_4d6e_8f3a_5e1c_77d0_a9b2);

/// The id named by `kind` and `name` (the same arguments give the same id).
pub(super) fn derived(kind: &str, name: &[u8]) -> ObjectId {
    let mut full = kind.as_bytes().to_vec();
    full.push(0);
    full.extend_from_slice(name);
    let named = Uuid::new_v5(&NAMESPACE, &full);
    let v7 = Builder::from_bytes(*named.as_bytes())
        .with_version(Version::SortRand)
        .with_variant(Variant::RFC4122)
        .into_uuid();
    ObjectId::from_uuid(v7).expect("a version 7 uuid is an object id")
}

/// The id of revision `number` of text `text`.
pub(super) fn revision_id(text: ObjectId, number: u64) -> ObjectId {
    let mut name = text.as_uuid().as_bytes().to_vec();
    name.extend_from_slice(&number.to_be_bytes());
    derived("revision", &name)
}

/// The id of the order list of the children of `parent` (none = top level).
pub(super) fn order_id(parent: Option<ObjectId>) -> ObjectId {
    let name = parent.map_or_else(Vec::new, |p| p.as_uuid().as_bytes().to_vec());
    derived("order", &name)
}
