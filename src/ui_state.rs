use crate::{
    backup::{app_storage_root_dir, atomic_write_text},
    models::prettify_model_name,
    types::{
        CatalogModel, CustomProviderProfile, LayoutDensity, SupportedProtocol, ThemeMode,
        UiStateSnapshot,
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::PathBuf};

const UI_STATE_FILE_NAME: &str = "ui-state.toml";

#[derive(Debug, Default, Deserialize, Serialize)]
struct PersistedUiState {
    #[serde(default)]
    ui: PersistedUiSection,
    #[serde(default)]
    custom_provider_profiles: Vec<PersistedCustomProviderProfile>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedUiSection {
    #[serde(default)]
    layout_density: LayoutDensity,
    #[serde(default)]
    theme_mode: ThemeMode,
}

impl Default for PersistedUiSection {
    fn default() -> Self {
        Self {
            layout_density: LayoutDensity::default(),
            theme_mode: ThemeMode::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedCustomProviderProfile {
    profile_id: String,
    label: String,
    base_url: String,
    env_key: String,
    protocol: PersistedSupportedProtocol,
    #[serde(default)]
    models: Vec<PersistedCatalogModel>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum PersistedSupportedProtocol {
    Openai,
    Anthropic,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct PersistedCatalogModel {
    id: String,
    name: String,
    context_window_size: Option<u64>,
    #[serde(default)]
    supports_vision: bool,
}

pub(crate) fn load_ui_state_snapshot() -> Result<UiStateSnapshot, String> {
    let path = ui_state_file_path()?;
    let persisted = load_persisted_ui_state(&path)?;
    Ok(snapshot_from_persisted(persisted))
}

pub(crate) fn save_ui_state_snapshot(
    layout_density: LayoutDensity,
    theme_mode: ThemeMode,
    custom_provider_profiles: Vec<CustomProviderProfile>,
) -> Result<UiStateSnapshot, String> {
    let snapshot = UiStateSnapshot {
        layout_density,
        theme_mode,
        custom_provider_profiles: normalize_custom_provider_profiles(custom_provider_profiles),
    };
    let persisted = persisted_from_snapshot(&snapshot);
    let toml_text = toml::to_string_pretty(&persisted)
        .map_err(|error| format!("Failed to serialize UI state TOML: {error}"))?;
    atomic_write_text(&ui_state_file_path()?, &toml_text)?;
    Ok(snapshot)
}

fn load_persisted_ui_state(path: &PathBuf) -> Result<PersistedUiState, String> {
    match fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents)
            .map_err(|error| format!("Failed to parse UI state TOML {}: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PersistedUiState::default())
        }
        Err(error) => Err(format!(
            "Failed to read UI state {}: {error}",
            path.display()
        )),
    }
}

fn ui_state_file_path() -> Result<PathBuf, String> {
    Ok(app_storage_root_dir()?.join(UI_STATE_FILE_NAME))
}

fn snapshot_from_persisted(persisted: PersistedUiState) -> UiStateSnapshot {
    UiStateSnapshot {
        layout_density: persisted.ui.layout_density,
        theme_mode: persisted.ui.theme_mode,
        custom_provider_profiles: normalize_custom_provider_profiles(
            persisted
                .custom_provider_profiles
                .into_iter()
                .map(|profile| CustomProviderProfile {
                    profile_id: profile.profile_id,
                    label: profile.label,
                    base_url: profile.base_url,
                    env_key: profile.env_key,
                    protocol: match profile.protocol {
                        PersistedSupportedProtocol::Openai => SupportedProtocol::Openai,
                        PersistedSupportedProtocol::Anthropic => SupportedProtocol::Anthropic,
                    },
                    models: profile
                        .models
                        .into_iter()
                        .map(|model| CatalogModel {
                            id: model.id,
                            name: model.name,
                            context_window_size: model.context_window_size,
                            supports_vision: model.supports_vision,
                        })
                        .collect(),
                })
                .collect(),
        ),
    }
}

fn normalize_custom_provider_profiles(
    profiles: Vec<CustomProviderProfile>,
) -> Vec<CustomProviderProfile> {
    let mut seen_ids = BTreeSet::new();
    let mut next_generated_index = 1usize;

    profiles
        .into_iter()
        .map(|profile| {
            let mut normalized_profile = CustomProviderProfile {
                profile_id: normalize_profile_id(
                    &profile.profile_id,
                    &mut seen_ids,
                    &mut next_generated_index,
                ),
                label: profile.label.trim().to_string(),
                base_url: profile.base_url.trim().to_string(),
                env_key: profile.env_key.trim().to_string(),
                protocol: profile.protocol,
                models: normalize_catalog_models(profile.models),
            };

            if normalized_profile.label.is_empty() {
                normalized_profile.label = format!("Custom Provider {}", seen_ids.len());
            }

            normalized_profile
        })
        .collect()
}

fn normalize_profile_id(
    value: &str,
    seen_ids: &mut BTreeSet<String>,
    next_generated_index: &mut usize,
) -> String {
    let trimmed = value.trim();
    if !trimmed.is_empty() && seen_ids.insert(trimmed.to_string()) {
        return trimmed.to_string();
    }

    loop {
        let candidate = format!("custom-provider-{}", *next_generated_index);
        *next_generated_index += 1;
        if seen_ids.insert(candidate.clone()) {
            return candidate;
        }
    }
}

fn normalize_catalog_models(models: Vec<CatalogModel>) -> Vec<CatalogModel> {
    let mut seen_ids = BTreeSet::new();

    models
        .into_iter()
        .filter_map(|model| {
            let id = model.id.trim().to_string();
            if id.is_empty() || !seen_ids.insert(id.clone()) {
                return None;
            }

            let name = {
                let trimmed = model.name.trim();
                if trimmed.is_empty() {
                    prettify_model_name(&id)
                } else {
                    trimmed.to_string()
                }
            };

            Some(CatalogModel {
                id,
                name,
                context_window_size: model.context_window_size,
                supports_vision: model.supports_vision,
            })
        })
        .collect()
}

fn persisted_from_snapshot(snapshot: &UiStateSnapshot) -> PersistedUiState {
    PersistedUiState {
        ui: PersistedUiSection {
            layout_density: snapshot.layout_density,
            theme_mode: snapshot.theme_mode,
        },
        custom_provider_profiles: snapshot
            .custom_provider_profiles
            .iter()
            .map(|profile| PersistedCustomProviderProfile {
                profile_id: profile.profile_id.clone(),
                label: profile.label.clone(),
                base_url: profile.base_url.clone(),
                env_key: profile.env_key.clone(),
                protocol: match profile.protocol {
                    SupportedProtocol::Openai => PersistedSupportedProtocol::Openai,
                    SupportedProtocol::Anthropic => PersistedSupportedProtocol::Anthropic,
                },
                models: profile
                    .models
                    .iter()
                    .map(|model| PersistedCatalogModel {
                        id: model.id.clone(),
                        name: model.name.clone(),
                        context_window_size: model.context_window_size,
                        supports_vision: model.supports_vision,
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_defaults_when_file_is_empty_or_missing() {
        let snapshot = snapshot_from_persisted(PersistedUiState::default());
        assert_eq!(snapshot.layout_density, LayoutDensity::Compact);
        assert_eq!(snapshot.theme_mode, ThemeMode::System);
        assert!(snapshot.custom_provider_profiles.is_empty());
    }

    #[test]
    fn layout_density_deserialization_rejects_unknown_values() {
        assert_eq!(
            LayoutDensity::parse("comfortable"),
            Some(LayoutDensity::Comfortable)
        );
        assert_eq!(LayoutDensity::parse("dense"), None);

        let parsed: PersistedUiSection =
            toml::from_str("layout_density = \"dense\"\ntheme_mode = \"dark\"").unwrap();
        assert_eq!(parsed.layout_density, LayoutDensity::Compact);
    }

    #[test]
    fn theme_mode_deserialization_rejects_unknown_values() {
        assert_eq!(ThemeMode::parse("dark"), Some(ThemeMode::Dark));
        assert_eq!(ThemeMode::parse("sepia"), None);

        let parsed: PersistedUiSection =
            toml::from_str("layout_density = \"compact\"\ntheme_mode = \"sepia\"").unwrap();
        assert_eq!(parsed.theme_mode, ThemeMode::System);
    }

    #[test]
    fn normalize_custom_provider_profiles_trims_and_deduplicates_models() {
        let normalized = normalize_custom_provider_profiles(vec![CustomProviderProfile {
            profile_id: " custom-a ".to_string(),
            label: "  ".to_string(),
            base_url: " https://example.com/v1/ ".to_string(),
            env_key: " EXAMPLE_KEY ".to_string(),
            protocol: SupportedProtocol::Openai,
            models: vec![
                CatalogModel {
                    id: " model-a ".to_string(),
                    name: String::new(),
                    context_window_size: Some(42),
                    supports_vision: false,
                },
                CatalogModel {
                    id: "model-a".to_string(),
                    name: "Duplicate".to_string(),
                    context_window_size: None,
                    supports_vision: true,
                },
                CatalogModel {
                    id: "   ".to_string(),
                    name: "Ignored".to_string(),
                    context_window_size: None,
                    supports_vision: false,
                },
            ],
        }]);

        assert_eq!(normalized.len(), 1);
        assert_eq!(normalized[0].profile_id, "custom-a");
        assert_eq!(normalized[0].label, "Custom Provider 1");
        assert_eq!(normalized[0].base_url, "https://example.com/v1/");
        assert_eq!(normalized[0].env_key, "EXAMPLE_KEY");
        assert_eq!(normalized[0].models.len(), 1);
        assert_eq!(normalized[0].models[0].id, "model-a");
        assert_eq!(normalized[0].models[0].name, "Model A");
    }

    #[test]
    fn normalize_custom_provider_profiles_generates_unique_profile_ids() {
        let normalized = normalize_custom_provider_profiles(vec![
            CustomProviderProfile {
                profile_id: String::new(),
                label: "One".to_string(),
                base_url: String::new(),
                env_key: String::new(),
                protocol: SupportedProtocol::Openai,
                models: Vec::new(),
            },
            CustomProviderProfile {
                profile_id: String::new(),
                label: "Two".to_string(),
                base_url: String::new(),
                env_key: String::new(),
                protocol: SupportedProtocol::Anthropic,
                models: Vec::new(),
            },
        ]);

        assert_eq!(normalized[0].profile_id, "custom-provider-1");
        assert_eq!(normalized[1].profile_id, "custom-provider-2");
    }
}
