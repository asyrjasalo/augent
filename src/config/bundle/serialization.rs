//! Serialization implementations for BundleConfig

use crate::config::utils::count_optional_fields;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serializer};

macro_rules! serialize_optional_field {
    ($state:expr, $name:expr, $value:expr) => {
        if let Some(val) = $value {
            $state.serialize_field($name, val)?;
        }
    };
}

/// Serialize BundleConfig (empty name field, name injected externally)
pub fn serialize_bundle_config<S>(
    _config: &BundleConfigData,
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
    } = _config;

    let optional_count = count_optional_fields(description, version, author, license, homepage);
    let field_count = 2 + optional_count;

    let mut state = serializer.serialize_struct("BundleConfig", field_count)?;

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
    #[derive(serde::Deserialize)]
    struct Raw {
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        author: Option<String>,
        #[serde(default)]
        license: Option<String>,
        #[serde(default)]
        homepage: Option<String>,
        #[serde(default)]
        bundles: Vec<super::dependency::BundleDependency>,
    }

    let raw = Raw::deserialize(deserializer)?;
    Ok(BundleConfigData {
        description: raw.description,
        version: raw.version,
        author: raw.author,
        license: raw.license,
        homepage: raw.homepage,
        bundles: raw.bundles,
    })
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
