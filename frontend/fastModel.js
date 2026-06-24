import { elements, state } from "./shared.js";
import { prettifyModelName } from "./utils.js";

let deps = null;

export function initializeFastModel(nextDeps) {
  deps = nextDeps;

  elements.fastModelSelect.addEventListener("change", () => {
    const [protocol, modelId] = decodeFastModelKey(elements.fastModelSelect.value);
    state.fastModel =
      protocol === null && modelId === null
        ? {
            mode: "inherit",
            protocol: null,
            modelId: null,
            rawValue: null,
          }
        : {
            mode: "specific",
            protocol,
            modelId,
            rawValue: null,
          };
    state.fastModelTouched = true;
    renderFastModelControls();
    deps.markStateChanged();
  });
}

export function buildFastModelRequest() {
  if (!state.fastModelTouched) {
    return undefined;
  }

  if (state.fastModel.mode === "inherit") {
    return {
      mode: "inherit",
      protocol: null,
      modelId: null,
      rawValue: null,
    };
  }

  if (state.fastModel.mode === "specific") {
    return {
      mode: "specific",
      protocol: state.fastModel.protocol,
      modelId: state.fastModel.modelId,
      rawValue: null,
    };
  }

  return undefined;
}

export function normalizeFastModelSnapshot(value) {
  return {
    mode: value?.mode ?? "inherit",
    protocol: value?.protocol ?? null,
    modelId: value?.modelId ?? null,
    rawValue: value?.rawValue ?? null,
  };
}

export function renderFastModelControls() {
  const options = buildFastModelOptions();
  const hasValidSpecificSelection =
    state.fastModel.mode === "specific" &&
    options.some(
      (option) =>
        option.protocol === state.fastModel.protocol &&
        option.modelId === state.fastModel.modelId,
    );
  const isInvalidSavedValue = state.fastModel.mode === "invalid";
  const hasMissingSpecificSelection =
    state.fastModel.mode === "specific" && !hasValidSpecificSelection;

  elements.fastModelSelect.replaceChildren();

  const inheritOption = document.createElement("option");
  inheritOption.value = encodeFastModelKey(null, null);
  inheritOption.textContent = "Use main model";
  elements.fastModelSelect.append(inheritOption);

  options.forEach((option) => {
    const element = document.createElement("option");
    element.value = encodeFastModelKey(option.protocol, option.modelId);
    element.textContent = option.label;
    elements.fastModelSelect.append(element);
  });

  if (isInvalidSavedValue || hasMissingSpecificSelection) {
    const missingOption = document.createElement("option");
    missingOption.value = encodeFastModelKey(
      state.fastModel.protocol,
      state.fastModel.modelId,
    );
    missingOption.textContent = state.fastModel.rawValue
      ? `Invalid saved value: ${state.fastModel.rawValue}`
      : formatFastModelLabel(
          state.fastModel.protocol,
          state.fastModel.modelId,
          "Not configured",
        );
    elements.fastModelSelect.append(missingOption);
  }

  elements.fastModelSelect.value =
    state.fastModel.mode === "inherit"
      ? encodeFastModelKey(null, null)
      : encodeFastModelKey(state.fastModel.protocol, state.fastModel.modelId);
  elements.fastModelSelect.disabled = false;

  elements.fastModelNote.textContent = buildFastModelNote(
    options,
    hasValidSpecificSelection,
  );
}

function buildFastModelOptions() {
  const seen = new Set();
  const options = [];

  state.models.forEach((model) => {
    const key = encodeFastModelKey(model.protocol, model.id);
    if (seen.has(key)) {
      return;
    }
    seen.add(key);
    options.push({
      protocol: model.protocol,
      modelId: model.id,
      label: formatFastModelLabel(
        model.protocol,
        model.id,
        normalizedDisplayName(model),
      ),
    });
  });

  return options;
}

function buildFastModelNote(options, hasValidSpecificSelection) {
  if (state.fastModel.mode === "invalid") {
    return state.fastModel.rawValue
      ? `Saved value ${state.fastModel.rawValue} is invalid and will be preserved until changed.`
      : "Saved fast model value is invalid and will be preserved until changed.";
  }

  if (state.fastModel.mode === "inherit") {
    return "Using the main model for fast-model features.";
  }

  if (!hasValidSpecificSelection) {
    return "This saved fast model does not match a configured model entry shown here.";
  }

  const defaultModel = state.models.find((model) => model.isDefault);
  if (
    defaultModel &&
    defaultModel.protocol === state.fastModel.protocol &&
    defaultModel.id === state.fastModel.modelId
  ) {
    return "Pinned to the current main model. It will stay here if the main model changes.";
  }

  if (options.length === 0) {
    return "Add a configured model before selecting a fast model override.";
  }

  return "Model for generating prompt suggestions and speculative execution.";
}

function normalizedDisplayName(model) {
  const trimmed = String(model.name || "").trim();
  return trimmed || prettifyModelName(model.id);
}

function formatFastModelLabel(protocol, modelId, displayName) {
  const safeProtocol = protocol || "unknown";
  const safeModelId = modelId || "unknown";
  return `${displayName || safeModelId}  •  ${safeProtocol}  •  ${safeModelId}`;
}

function encodeFastModelKey(protocol, modelId) {
  return `${protocol || ""}\u0000${modelId || ""}`;
}

function decodeFastModelKey(value) {
  const [protocol, modelId] = String(value || "").split("\u0000");
  return [protocol || null, modelId || null];
}
