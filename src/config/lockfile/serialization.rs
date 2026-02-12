//! Serialization implementations for Lockfile

use serde::de::MapAccess;
use serde::de::Visitor;
use serde::ser::SerializeStruct;
use serde::{Deserializer, Serializer};
use std::fmt;

use crate::config::lockfile::bundle::LockedBundle;

/// Serialize Lockfile (empty name field, name injected externally)
pub fn serialize_lockfile<S>(
    bundles: &[LockedBundle],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut state = serializer.serialize_struct("Lockfile", 2)?;
    state.serialize_field("name", "")?;
    state.serialize_field("bundles", bundles)?;
    state.end()
}

/// Deserialize Lockfile (skip name field, read from filesystem)
pub fn deserialize_lockfile<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<LockedBundle>, D::Error>
where
    D: Deserializer<'de>,
{
    struct LockfileVisitor;

    impl<'de> Visitor<'de> for LockfileVisitor {
        type Value = Vec<LockedBundle>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a Lockfile")
        }

        fn visit_map<M>(self, map: M) -> std::result::Result<Vec<LockedBundle>, M::Error>
        where
            M: MapAccess<'de>,
        {
            process_map(map)
        }
    }

    deserializer.deserialize_map(LockfileVisitor)
}

fn process_map<'de, M>(mut map: M) -> std::result::Result<Vec<LockedBundle>, M::Error>
where
    M: MapAccess<'de>,
{
    let mut bundles = None;
    while let Some(key) = map.next_key::<String>()? {
        bundles = Some(process_map_key(key.as_str(), &mut map, bundles)?);
    }
    Ok(bundles.unwrap_or_default())
}

fn process_map_key<'de, M>(
    key: &str,
    map: &mut M,
    bundles: Option<Vec<LockedBundle>>,
) -> std::result::Result<Vec<LockedBundle>, M::Error>
where
    M: MapAccess<'de>,
{
    if key != "bundles" {
        map.next_value::<serde::de::IgnoredAny>()?;
        return Ok(bundles.unwrap_or_default());
    }
    map.next_value()
}
