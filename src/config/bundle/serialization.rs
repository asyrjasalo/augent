//! Serialization implementations for BundleConfig

use serde::de::MapAccess;
use serde::de::Visitor;
use serde::ser::SerializeStruct;
use serde::{Deserializer, Serializer};
use std::fmt;

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

    if let Some(description) = description {
        state.serialize_field("description", description)?;
    }
    if let Some(version) = version {
        state.serialize_field("version", version)?;
    }
    if let Some(author) = author {
        state.serialize_field("author", author)?;
    }
    if let Some(license) = license {
        state.serialize_field("license", license)?;
    }
    if let Some(homepage) = homepage {
        state.serialize_field("homepage", homepage)?;
    }
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

            while let Some(key) = map.next_key()? {
                let key: String = key;
                match key.as_str() {
                    "name" => {
                        // Skip name field - it's read from filesystem location
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                    "description" => {
                        description = map.next_value()?;
                    }
                    "version" => {
                        version = map.next_value()?;
                    }
                    "author" => {
                        author = map.next_value()?;
                    }
                    "license" => {
                        license = map.next_value()?;
                    }
                    "homepage" => {
                        homepage = map.next_value()?;
                    }
                    "bundles" => {
                        bundles = map.next_value()?;
                    }
                    _ => {
                        // Skip unknown fields
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                }
            }

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
