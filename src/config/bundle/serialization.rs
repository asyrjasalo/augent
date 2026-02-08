//! Serialization implementations for BundleConfig

use serde::de::MapAccess;
use serde::de::Visitor;
use serde::ser::SerializeStruct;
use serde::{Deserializer, Serializer};
use std::fmt;

macro_rules! serialize_optional_field {
    ($state:expr, $name:expr, $value:expr) => {
        if let Some(val) = $value {
            $state.serialize_field($name, val)?;
        }
    };
}

macro_rules! extract_map_field {
    ($map:ident, $key:literal, $var:ident) => {
        while let Some(key) = $map.next_key()? {
            let key: String = key;
            if key.as_str() == $key {
                $var = $map.next_value()?;
                break;
            } else {
                // Must consume the value before moving to next key
                let _: serde::de::IgnoredAny = $map.next_value()?;
            }
        }
    };
}

/// Serialize BundleConfig (empty name field, name injected externally)
pub fn serialize_bundle_config<S>(
    config: &BundleConfigData,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let BundleConfigData {
        description,
        version,
        author,
        license,
        homepage,
        bundles,
    } = config;

    // Count fields: name (always serialized) + optional fields + bundles
    let mut field_count = 2; // name + bundles
    if description.is_some() {
        field_count += 1;
    }
    if version.is_some() {
        field_count += 1;
    }
    if author.is_some() {
        field_count += 1;
    }
    if license.is_some() {
        field_count += 1;
    }
    if homepage.is_some() {
        field_count += 1;
    }

    let mut state = serializer.serialize_struct("BundleConfig", field_count)?;

    // Note: name is injected externally during file write, we serialize empty string
    state.serialize_field("name", "")?;

    serialize_optional_field!(state, "description", description);
    serialize_optional_field!(state, "version", version);
    serialize_optional_field!(state, "author", author);
    serialize_optional_field!(state, "license", license);
    serialize_optional_field!(state, "homepage", homepage);
    state.serialize_field("bundles", bundles)?;
    state.end()
}

/// Deserialize BundleConfig (skip name field, read from filesystem)
pub fn deserialize_bundle_config<'de, D>(
    deserializer: D,
) -> std::result::Result<BundleConfigData, D::Error>
where
    D: Deserializer<'de>,
{
    struct BundleConfigVisitor;

    impl<'de> Visitor<'de> for BundleConfigVisitor {
        type Value = BundleConfigData;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a BundleConfig")
        }

        fn visit_map<M>(self, mut map: M) -> std::result::Result<BundleConfigData, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut description = None;
            let mut version = None;
            let mut author = None;
            let mut license = None;
            let mut homepage = None;
            let mut bundles = Vec::new();

            extract_map_field!(map, "description", description);
            extract_map_field!(map, "version", version);
            extract_map_field!(map, "author", author);
            extract_map_field!(map, "license", license);
            extract_map_field!(map, "homepage", homepage);
            extract_map_field!(map, "bundles", bundles);

            Ok(BundleConfigData {
                description,
                version,
                author,
                license,
                homepage,
                bundles,
            })
        }
    }

    deserializer.deserialize_map(BundleConfigVisitor)
}

/// Internal struct to hold BundleConfig fields
pub struct BundleConfigData {
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub bundles: Vec<super::dependency::BundleDependency>,
}
