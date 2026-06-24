use serde::{Deserialize, Serialize, de::Deserializer, ser::Serializer};
use std::fmt;

use super::CustomProviderProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayoutDensity {
    Compact,
    Comfortable,
    Spacious,
}

impl LayoutDensity {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Comfortable => "comfortable",
            Self::Spacious => "spacious",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "compact" => Some(Self::Compact),
            "comfortable" => Some(Self::Comfortable),
            "spacious" => Some(Self::Spacious),
            _ => None,
        }
    }
}

impl Default for LayoutDensity {
    fn default() -> Self {
        Self::Compact
    }
}

impl Serialize for LayoutDensity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LayoutDensity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::parse(&value).unwrap_or_default())
    }
}

impl fmt::Display for LayoutDensity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeMode {
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "system" => Some(Self::System),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::System
    }
}

impl Serialize for ThemeMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ThemeMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(Self::parse(&value).unwrap_or_default())
    }
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiStateSnapshot {
    pub(crate) layout_density: LayoutDensity,
    pub(crate) theme_mode: ThemeMode,
    pub(crate) custom_provider_profiles: Vec<CustomProviderProfile>,
}
