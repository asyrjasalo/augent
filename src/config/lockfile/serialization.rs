//! Serialization implementations for Lockfile

use serde::ser::SerializeStruct;
use serde::{Deserializer, Serializer};

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
    // Note: name is injected externally during file write, we serialize empty string
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
    use serde::de::MapAccess;
    use serde::de::Visitor;
    use std::fmt;

    struct LockfileVisitor;

    impl<'de> Visitor<'de> for LockfileVisitor {
        type Value = Vec<LockedBundle>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a Lockfile")
        }

        fn visit_map<M>(self, mut map: M) -> std::result::Result<Vec<LockedBundle>, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut bundles = Vec::new();

            while let Some(key) = map.next_key::<String>()? {
                match key.as_str() {
                    "bundles" => bundles = map.next_value()?,
                    _ => {
                        drop(map.next_value::<serde::de::IgnoredAny>()?);
                    }
                }
            }

            Ok(bundles)
        }
    }

    deserializer.deserialize_map(LockfileVisitor)
}
