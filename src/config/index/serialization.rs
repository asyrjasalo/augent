//! Serialization implementations for `WorkspaceConfig`

use serde::de::MapAccess;
use serde::de::Visitor;
use serde::ser::SerializeStruct;
use serde::{Deserializer, Serializer};
use std::fmt;

/// Serialize `WorkspaceConfig` (empty name field, name injected externally)
pub fn serialize_workspace_config<S>(
    bundles: &[super::bundle::WorkspaceBundle],
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut state = serializer.serialize_struct("WorkspaceConfig", 2)?;
    state.serialize_field("name", "")?;
    state.serialize_field("bundles", bundles)?;
    state.end()
}

/// Deserialize `WorkspaceConfig` (skip name field, read from filesystem)
pub fn deserialize_workspace_config<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<super::bundle::WorkspaceBundle>, D::Error>
where
    D: Deserializer<'de>,
{
    struct WorkspaceConfigVisitor;

    impl<'de> Visitor<'de> for WorkspaceConfigVisitor {
        type Value = Vec<super::bundle::WorkspaceBundle>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a WorkspaceConfig")
        }

        fn visit_map<M>(
            self,
            map: M,
        ) -> std::result::Result<Vec<super::bundle::WorkspaceBundle>, M::Error>
        where
            M: MapAccess<'de>,
        {
            process_map(map)
        }
    }

    deserializer.deserialize_map(WorkspaceConfigVisitor)
}

fn process_map<'de, M>(
    mut map: M,
) -> std::result::Result<Vec<super::bundle::WorkspaceBundle>, M::Error>
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
    bundles: Option<Vec<super::bundle::WorkspaceBundle>>,
) -> std::result::Result<Vec<super::bundle::WorkspaceBundle>, M::Error>
where
    M: MapAccess<'de>,
{
    if key != "bundles" {
        map.next_value::<serde::de::IgnoredAny>()?;
        return Ok(bundles.unwrap_or_default());
    }
    map.next_value()
}
