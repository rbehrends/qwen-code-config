import { invoke } from "./shared.js";

function requireInvoke() {
  if (!invoke) {
    throw new Error("Tauri API unavailable");
  }

  return invoke;
}

export function hasTauriApi() {
  return Boolean(invoke);
}

export async function loadSettingsSnapshot(path) {
  return requireInvoke()("load_settings", { path });
}

export async function loadUiStateSnapshot() {
  return requireInvoke()("load_ui_state");
}

export async function getPendingOpenPath() {
  return requireInvoke()("get_pending_open_path");
}

export async function saveOptions(request) {
  return requireInvoke()("save_options", { request });
}

export async function saveOptionsAs(request) {
  return requireInvoke()("save_options_as", { request });
}

export async function fetchProviderModels(providerId) {
  return requireInvoke()("fetch_provider_models", {
    request: { providerId },
  });
}

export async function buildModelDraft(profile, source, hasExistingDefault) {
  // This payload is transport mapping only. Rust owns provider-specific draft
  // construction and any canonical normalization of the resulting model row.
  const result = await requireInvoke()("build_model_draft_command", {
    request: {
      profile: {
        baseUrl: profile.baseUrl,
        envKey: profile.envKey,
        protocol: profile.protocol,
      },
      model: {
        id: source.id,
        name: source.name,
        contextWindowSize: source.contextWindowSize ?? null,
        supportsVision: Boolean(source.supportsVision),
      },
      hasExistingDefault,
    },
  });

  return result?.model;
}

export async function getModelNameRewriteRules() {
  return requireInvoke()("get_model_name_rewrite_rules");
}

export async function previewSettings(request) {
  return requireInvoke()("preview_settings_command", { request });
}

export async function syncMenuUiState(request) {
  return requireInvoke()("sync_menu_ui_state", { request });
}

export async function saveUiState(request) {
  return requireInvoke()("save_ui_state", { request });
}

export async function quitApp() {
  return requireInvoke()("quit_app");
}
