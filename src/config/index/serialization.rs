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
    // Note: name is injected externally during file write, we serialize empty string
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
            mut map: M,
        ) -> std::result::Result<Vec<super::bundle::WorkspaceBundle>, M::Error>
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

    deserializer.deserialize_map(WorkspaceConfigVisitor)
}
