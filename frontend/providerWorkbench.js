import { elements, state } from "./shared.js";
import { filterCatalogModels, prettifyModelName } from "./utils.js";

let deps = null;
let eventsBound = false;

export function initializeProviderWorkbench(nextDeps) {
  deps = nextDeps;

  if (eventsBound) {
    return;
  }

  eventsBound = true;
  bindSelectedProviderProfileField(
    elements.providerLabelInput,
    "input",
    "label",
    {
      rerenderProfiles: true,
    },
  );
  bindSelectedProviderProfileField(
    elements.providerProtocolSelect,
    "change",
    "protocol",
  );
  bindSelectedProviderProfileField(
    elements.providerBaseUrlInput,
    "input",
    "baseUrl",
  );
  bindSelectedProviderProfileField(
    elements.providerEnvKeyInput,
    "input",
    "envKey",
  );

  elements.providerSelect.addEventListener("change", () => {
    state.selectedProviderProfileId = elements.providerSelect.value || null;
    renderProviderWorkbench();
  });

  elements.newProviderButton.addEventListener("click", () => {
    const profile = {
      // This id only needs to be locally unique while editing. Rust still
      // canonicalizes persisted custom profile ids when UI state is saved.
      profileId: `custom-${Date.now()}-${Math.random().toString(16).slice(2, 7)}`,
      presetId: null,
      label: `Custom Provider ${countCustomProviders() + 1}`,
      baseUrl: "",
      envKey: "",
      protocol: "openai",
      fetchedModels: [],
      customModels: [],
    };
    state.providerProfiles.push(profile);
    state.selectedProviderProfileId = profile.profileId;
    renderProviderProfiles();
    renderProviderWorkbench();
    deps.queueUiStatePersist();
  });

  elements.removeProviderButton.addEventListener("click", async () => {
    const profile = getSelectedProviderProfile();
    if (!profile || profile.presetId) {
      return;
    }

    if (!(await deps.confirmRemoveCustomProvider(profile.profileId))) {
      return;
    }

    const index = state.providerProfiles.findIndex(
      (candidate) => candidate.profileId === profile.profileId,
    );
    if (index === -1) {
      return;
    }

    state.providerProfiles.splice(index, 1);
    state.selectedProviderProfileId =
      state.providerProfiles[0]?.profileId ?? null;
    renderProviderProfiles();
    renderProviderWorkbench();
    deps.queueUiStatePersist();
  });

  elements.fetchProviderModelsButton.addEventListener("click", async () => {
    const profile = getSelectedProviderProfile();
    if (!profile || !profile.presetId) {
      deps.setStatus("Only built-in providers can fetch catalogs", "error");
      return;
    }

    deps.setStatus("Fetching models", "");

    try {
      const result = await deps.fetchProviderModels(profile.presetId);
      profile.fetchedModels = result.models;
      renderCatalog();
      deps.setStatus("Catalog updated", "ready");
    } catch (error) {
      deps.setStatus(error, "error");
    }
  });

  elements.addManualModelButton.addEventListener("click", async () => {
    const profile = getSelectedProviderProfile();
    if (!profile) {
      deps.setStatus("Select a provider profile first", "error");
      return;
    }

    const modelId = elements.manualModelIdInput.value.trim();
    const modelName = elements.manualModelNameInput.value.trim();
    if (!modelId) {
      deps.setStatus("Manual model id is required", "error");
      return;
    }

    if (!profile.presetId) {
      if (catalogContainsModel(profile, modelId)) {
        deps.setStatus(
          "That model already exists in this custom provider catalog",
          "error",
        );
        return;
      }

      addCustomCatalogModel(profile, modelId, modelName);
      elements.manualModelIdInput.value = "";
      elements.manualModelNameInput.value = "";
      deps.setStatus("Saved to provider catalog", "ready");
      return;
    }

    if (
      deps.hasPotentialDuplicateConfiguredModel(
        profile.protocol,
        modelId,
        profile.baseUrl,
      )
    ) {
      deps.setStatus(
        "That model already exists for the same protocol and base URL",
        "error",
      );
      return;
    }

    try {
      await deps.addConfiguredModelFromSource(profile, {
        id: modelId,
        name: modelName,
        contextWindowSize: null,
        supportsVision: false,
      });
      elements.manualModelIdInput.value = "";
      elements.manualModelNameInput.value = "";
    } catch (error) {
      deps.setStatus(error, "error");
    }
  });

  elements.catalogFilterInput.addEventListener("input", () => {
    renderCatalog();
  });

  elements.catalogFilterMode.addEventListener("change", () => {
    renderCatalog();
  });
}

function bindSelectedProviderProfileField(
  element,
  eventName,
  key,
  { rerenderProfiles = false } = {},
) {
  element.addEventListener(eventName, () => {
    const profile = getSelectedProviderProfile();
    if (!profile) {
      return;
    }

    profile[key] = element.value;
    if (rerenderProfiles) {
      renderProviderProfiles();
    }
    renderCatalog();
    if (!profile.presetId) {
      deps.queueUiStatePersist();
    }
  });
}

export function getSelectedProviderProfile() {
  return (
    state.providerProfiles.find(
      (profile) => profile.profileId === state.selectedProviderProfileId,
    ) ?? null
  );
}

export function buildInitialProviderProfiles() {
  const presetProfiles = state.providerPresets.map((preset) => ({
    profileId: `preset-${preset.id}`,
    presetId: preset.id,
    label: preset.label,
    baseUrl: preset.baseUrl,
    envKey: preset.defaultEnvKey,
    protocol: preset.defaultProtocol,
    fetchedModels: [],
    customModels: [],
  }));

  const customProfiles = state.customProviderProfiles.map((profile) => ({
    profileId: profile.profileId,
    presetId: null,
    label: profile.label,
    baseUrl: profile.baseUrl,
    envKey: profile.envKey,
    protocol: profile.protocol,
    fetchedModels: [],
    customModels: Array.isArray(profile.models) ? profile.models : [],
  }));

  return [...presetProfiles, ...customProfiles];
}

export function renderProviderProfiles() {
  elements.providerSelect.replaceChildren();

  state.providerProfiles.forEach((profile) => {
    const option = document.createElement("option");
    option.value = profile.profileId;
    option.textContent = profile.label || "Untitled provider";
    option.selected = profile.profileId === state.selectedProviderProfileId;
    elements.providerSelect.append(option);
  });
}

export function renderProviderWorkbench() {
  const profile = getSelectedProviderProfile();

  elements.removeProviderButton.disabled =
    !profile || Boolean(profile.presetId);
  elements.fetchProviderModelsButton.disabled = !profile || !profile.presetId;

  if (!profile) {
    elements.providerLabelInput.value = "";
    elements.providerProtocolSelect.value = "openai";
    elements.providerBaseUrlInput.value = "";
    elements.providerEnvKeyInput.value = "";
    elements.catalogSummary.textContent = "No provider selected";
    elements.addManualModelButton.textContent = "Add";
    elements.catalogList.replaceChildren();
    return;
  }

  elements.providerLabelInput.value = profile.label;
  elements.providerProtocolSelect.value = profile.protocol;
  elements.providerBaseUrlInput.value = profile.baseUrl;
  elements.providerEnvKeyInput.value = profile.envKey;
  elements.addManualModelButton.textContent = profile.presetId ? "Add" : "Save";

  renderCatalog();
}

export function renderCatalog() {
  const profile = getSelectedProviderProfile();
  elements.catalogList.replaceChildren();

  if (!profile) {
    elements.catalogSummary.textContent = "No provider selected";
    return;
  }

  const models = catalogModelsForProfile(profile);
  const filteredModels = filterCatalogModels(
    models,
    elements.catalogFilterInput.value,
    elements.catalogFilterMode.value,
  );
  elements.catalogSummary.textContent =
    models.length === 0
      ? profile.presetId
        ? "No fetched models"
        : "No saved models"
      : filteredModels.length === models.length
        ? profile.presetId
          ? `${models.length} fetched`
          : `${models.length} saved`
        : `${filteredModels.length} of ${models.length} shown`;

  if (models.length === 0) {
    const empty = document.createElement("div");
    empty.className = "catalog-empty";
    empty.textContent = profile.presetId
      ? "Fetch this provider catalog to add discovered models."
      : "Add manual model ids to build a reusable local catalog for this provider.";
    elements.catalogList.append(empty);
    return;
  }

  if (filteredModels.length === 0) {
    const empty = document.createElement("div");
    empty.className = "catalog-empty";
    empty.textContent = profile.presetId
      ? "No fetched models match the current filter."
      : "No saved models match the current filter.";
    elements.catalogList.append(empty);
    return;
  }

  filteredModels.forEach((catalogModel) => {
    const item = document.createElement("div");
    item.className = "catalog-item";

    const text = document.createElement("div");
    text.className = "catalog-text";

    const title = document.createElement("strong");
    title.textContent = displayCatalogModelName(catalogModel);

    const subtitle = document.createElement("small");
    const details = [catalogModel.id];
    if (catalogModel.contextWindowSize) {
      details.push(`${catalogModel.contextWindowSize.toLocaleString()} ctx`);
    }
    if (catalogModel.supportsVision) {
      details.push("vision");
    }
    subtitle.textContent = details.join("  •  ");

    text.append(title, subtitle);

    const actions = document.createElement("div");
    actions.className = "catalog-actions";

    const addButton = document.createElement("button");
    addButton.type = "button";
    addButton.className = "secondary-button compact-button";
    addButton.textContent = profile.presetId ? "Add" : "Use";
    addButton.title = profile.presetId
      ? "Add this model to the config"
      : "Use this model in the config";
    addButton.setAttribute("aria-label", addButton.title);
    addButton.disabled = deps.hasPotentialDuplicateConfiguredModel(
      profile.protocol,
      catalogModel.id,
      profile.baseUrl,
    );
    addButton.addEventListener("click", async () => {
      if (
        deps.hasPotentialDuplicateConfiguredModel(
          profile.protocol,
          catalogModel.id,
          profile.baseUrl,
        )
      ) {
        deps.setStatus(
          "That model already exists for the same protocol and base URL",
          "error",
        );
        return;
      }

      try {
        await deps.addConfiguredModelFromSource(profile, catalogModel);
      } catch (error) {
        deps.setStatus(error, "error");
      }
    });
    actions.append(addButton);

    if (!profile.presetId) {
      const removeButton = document.createElement("button");
      removeButton.type = "button";
      removeButton.className = "secondary-button compact-button danger-button";
      removeButton.textContent = "Discard";
      removeButton.title =
        "Discard this model from the custom provider catalog";
      removeButton.setAttribute("aria-label", removeButton.title);
      removeButton.addEventListener("click", () => {
        removeCustomCatalogModel(profile.profileId, catalogModel.id);
      });
      actions.append(removeButton);
    }

    item.append(text, actions);
    elements.catalogList.append(item);
  });
}

function catalogModelsForProfile(profile) {
  if (!profile) {
    return [];
  }

  return profile.presetId
    ? Array.isArray(profile.fetchedModels)
      ? profile.fetchedModels
      : []
    : Array.isArray(profile.customModels)
      ? profile.customModels
      : [];
}

function catalogContainsModel(profile, modelId) {
  return catalogModelsForProfile(profile).some(
    (model) => model.id.trim() === modelId.trim(),
  );
}

function displayCatalogModelName(model) {
  const trimmed = String(model?.name || "").trim();
  return trimmed || prettifyModelName(model?.id);
}

export function customProviderProfilesForSave() {
  // The frontend persists only editable profile data here. Rust remains the
  // authority for trimming, deduping catalog entries, and canonical ids.
  return state.providerProfiles
    .filter((profile) => !profile.presetId)
    .map((profile) => ({
      profileId: profile.profileId,
      label: profile.label,
      baseUrl: profile.baseUrl,
      envKey: profile.envKey,
      protocol: profile.protocol,
      models: Array.isArray(profile.customModels) ? profile.customModels : [],
    }));
}

function removeCustomCatalogModel(profileId, modelId) {
  const profile = state.providerProfiles.find(
    (candidate) => candidate.profileId === profileId && !candidate.presetId,
  );
  if (!profile || !Array.isArray(profile.customModels)) {
    return;
  }

  profile.customModels = profile.customModels.filter(
    (model) => model.id !== modelId,
  );
  renderCatalog();
  deps.queueUiStatePersist();
}

function addCustomCatalogModel(profile, modelId, modelName = "") {
  if (profile.presetId) {
    return;
  }

  if (!Array.isArray(profile.customModels)) {
    profile.customModels = [];
  }

  const trimmedId = modelId.trim();
  if (!trimmedId) {
    return;
  }

  if (profile.customModels.some((model) => model.id.trim() === trimmedId)) {
    return;
  }

  profile.customModels.push({
    id: trimmedId,
    name: modelName.trim(),
    contextWindowSize: null,
    supportsVision: false,
  });
  // Local custom catalog names may stay blank while editing; the fallback
  // prettifier is only for display, while Rust normalizes persisted names.
  renderCatalog();
  deps.queueUiStatePersist();
}

function countCustomProviders() {
  return state.providerProfiles.filter((profile) => !profile.presetId).length;
}
