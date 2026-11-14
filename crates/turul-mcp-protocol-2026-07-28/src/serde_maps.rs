//! Serialize `HashMap` wire fields in sorted key order (ADR-036).
//!
//! A `HashMap` iterates in a random per-map order, so a wire type holding one
//! would serialize the same value to different bytes. Every `HashMap` field of
//! a `Serialize` type in this crate goes through [`sorted`] or [`sorted_opt`].

use serde::{Serialize, Serializer};
use std::collections::HashMap;

/// Serialize a map with its entries in ascending key order.
pub fn sorted<K, V, H, S>(map: &HashMap<K, V, H>, serializer: S) -> Result<S::Ok, S::Error>
where
    K: Ord + Serialize,
    V: Serialize,
    S: Serializer,
{
    serializer.collect_map(sorted_entries(map))
}

/// A map's entries in ascending key order, for hand-written `Serialize` impls.
pub fn sorted_entries<K: Ord, V, H>(map: &HashMap<K, V, H>) -> Vec<(&K, &V)> {
    let mut entries: Vec<(&K, &V)> = map.iter().collect();
    entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
    entries
}

/// [`sorted`] for an `Option<HashMap<..>>` field.
pub fn sorted_opt<K, V, H, S>(
    map: &Option<HashMap<K, V, H>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    K: Ord + Serialize,
    V: Serialize,
    S: Serializer,
{
    match map {
        Some(map) => sorted(map, serializer),
        None => serializer.serialize_none(),
    }
}
