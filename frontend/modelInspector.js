import {
  createField,
  createNumberField,
  createSelectField,
} from "./domFields.js";
import { elements, state } from "./shared.js";
import { preservedKeysCount, prettifyModelName } from "./utils.js";
import { setDefaultModel } from "./modelReorder.js";

let deps = null;

const REASONING_MODE_CHOICES = [
  { value: "default", label: "Default" },
  { value: "enabled", label: "Enabled" },
  { value: "disabled", label: "Disabled" },
];
const OPENAI_REASONING_EFFORT_CHOICES = [
  { value: "", label: "Default" },
  { value: "minimal", label: "Minimal" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "xhigh", label: "X-High" },
  { value: "max", label: "Max" },
];
const ANTHROPIC_REASONING_EFFORT_CHOICES = [
  { value: "", label: "Default" },
  { value: "minimal", label: "Minimal" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "xhigh", label: "X-High" },
  { value: "max", label: "Max" },
];

export function initializeModelInspector(nextDeps) {
  deps = nextDeps;
}

export function renderModelInspector(model) {
  const preservedFocus = captureInspectorFocus(model);
  elements.modelInspectorContent.replaceChildren();

  if (!model) {
    const empty = document.createElement("div");
    empty.className = "model-inspector-empty";
    empty.textContent =
      state.models.length === 0
        ? "Add or fetch a model to edit its parameters."
        : "Select a model to inspect and edit its parameters.";
    elements.modelInspectorContent.append(empty);
    return;
  }

  const titleBlock = document.createElement("div");
  titleBlock.className = "inspector-title";

  const title = document.createElement("h3");
  title.textContent = normalizedDisplayName(model);

  const subtitle = document.createElement("p");
  subtitle.textContent = `${model.id}  •  ${deriveProviderBadge(model)}`;
  titleBlock.append(title, subtitle);

  const actionRow = document.createElement("div");
  actionRow.className = "inspector-actions";

  const defaultButton = document.createElement("button");
  defaultButton.type = "button";
  defaultButton.className = `secondary-button compact-button ${model.isDefault ? "selected-button" : ""}`;
  defaultButton.textContent = model.isDefault ? "Default" : "Make Default";
  defaultButton.addEventListener("click", () => {
    setDefaultModel(model.uiId);
  });

  const removeButton = document.createElement("button");
  removeButton.type = "button";
  removeButton.className = "secondary-button compact-button danger-button";
  removeButton.textContent = "Remove";
  removeButton.addEventListener("click", async () => {
    await deps.confirmAndRemoveModel(model.uiId);
  });

  actionRow.append(defaultButton, removeButton);

  const grid = document.createElement("div");
  grid.className = "model-grid";

  grid.append(
    createField(
      "Display Name",
      model.name,
      (value) => {
        model.name = value;
        syncModelPresentation(model);
        deps.markStateChanged();
      },
      {
        fieldKey: "name",
        placeholder: "Display name",
      },
    ),
  );

  grid.append(
    createField(
      "Model ID",
      model.id,
      (value) => {
        model.id = value;
        syncModelPresentation(model);
        deps.markStateChanged();
      },
      {
        fieldKey: "id",
        placeholder: "provider/model-id",
      },
    ),
  );

  grid.append(
    createSelectField(
      "Protocol",
      model.protocol,
      [
        { value: "openai", label: "OpenAI" },
        { value: "anthropic", label: "Anthropic" },
      ],
      (value) => {
        model.protocol = value;
        deps.renderModels();
        deps.markStateChanged();
      },
      { fieldKey: "protocol" },
    ),
  );

  grid.append(
    createField(
      "Env Key",
      model.envKey,
      (value) => {
        model.envKey = value;
        deps.markStateChanged();
      },
      { fieldKey: "envKey" },
    ),
  );

  grid.append(
    createField(
      "Base URL",
      model.baseUrl,
      (value) => {
        model.baseUrl = value;
        syncModelPresentation(model);
        deps.markStateChanged();
      },
      { fieldKey: "baseUrl" },
    ),
  );

  grid.append(
    createNumberField(
      "Context Window",
      model.contextWindowSize,
      (value) => {
        model.contextWindowSize = value;
        deps.markStateChanged();
      },
      { fieldKey: "contextWindowSize", integer: true },
    ),
  );

  grid.append(
    createNumberField(
      "Temperature",
      model.temperature,
      (value) => {
        model.temperature = value;
        deps.markStateChanged();
      },
      { fieldKey: "temperature" },
    ),
  );

  grid.append(
    createNumberField(
      "Top P",
      model.topP,
      (value) => {
        model.topP = value;
        deps.markStateChanged();
      },
      { fieldKey: "topP" },
    ),
  );

  grid.append(
    createNumberField(
      "Max Tokens",
      model.maxTokens,
      (value) => {
        model.maxTokens = value;
        deps.markStateChanged();
      },
      { fieldKey: "maxTokens", integer: true },
    ),
  );

  grid.append(
    createSelectField(
      "Reasoning Mode",
      model.reasoningMode ?? "default",
      REASONING_MODE_CHOICES,
      (value) => {
        model.reasoningMode = value || "default";
        deps.markStateChanged();
      },
      { fieldKey: "reasoningMode" },
    ),
  );

  grid.append(
    createSelectField(
      "Reasoning Effort",
      model.reasoningEffort ?? "",
      model.protocol === "anthropic"
        ? ANTHROPIC_REASONING_EFFORT_CHOICES
        : OPENAI_REASONING_EFFORT_CHOICES,
      (value) => {
        model.reasoningEffort = value || null;
        deps.markStateChanged();
      },
      { fieldKey: "reasoningEffort" },
    ),
  );

  if (model.protocol === "anthropic") {
    grid.append(
      createNumberField(
        "Reasoning Budget",
        model.reasoningBudgetTokens,
        (value) => {
          model.reasoningBudgetTokens = value;
          deps.markStateChanged();
        },
        { fieldKey: "reasoningBudgetTokens", integer: true },
      ),
    );
  }

  const footer = document.createElement("div");
  footer.className = "model-card-footer";
  const preservedSampling = preservedKeysCount(model.samplingParams, [
    "temperature",
    "top_p",
    "max_tokens",
  ]);
  const preservedExtraBody = Object.keys(model.extraBody || {}).length;
  const preservedReasoning = preservedKeysCount(
    model.rawModel?.generationConfig?.reasoning,
    ["enabled", "effort", "budget_tokens"],
  );
  footer.textContent =
    preservedSampling > 0 || preservedExtraBody > 0 || preservedReasoning > 0
      ? `Preserving ${preservedSampling} extra sampling key${preservedSampling === 1 ? "" : "s"}, ${preservedExtraBody} extra extra_body key${preservedExtraBody === 1 ? "" : "s"}, and ${preservedReasoning} extra reasoning key${preservedReasoning === 1 ? "" : "s"}.`
      : "Unknown provider fields are preserved on save.";

  const compatibilityNote = document.createElement("div");
  compatibilityNote.className = "model-card-footer";
  compatibilityNote.textContent =
    "Not all parameters or settings are accepted by every API or model.";

  elements.modelInspectorContent.append(
    titleBlock,
    actionRow,
    grid,
    footer,
    compatibilityNote,
  );

  restoreInspectorFocus(preservedFocus);
}

export function normalizedDisplayName(model) {
  const trimmed = String(model.name || "").trim();
  return trimmed || prettifyModelName(model.id);
}

export function deriveProviderBadge(model) {
  // This badge is presentation-only. Provider semantics such as catalog
  // fetching, draft construction, and preset metadata stay Rust-owned.
  const trimmedBaseUrl = normalizedProviderBaseUrl(model.baseUrl);
  const preset = state.providerPresets.find(
    (candidate) => candidate.baseUrl === trimmedBaseUrl,
  );

  if (preset) {
    return `${preset.label}  •  ${model.protocol}`;
  }

  return trimmedBaseUrl
    ? `${trimmedBaseUrl}  •  ${model.protocol}`
    : model.protocol;
}

export function syncModelPresentation(model) {
  const row = findModelRowElement(model.uiId);
  if (row) {
    const title = row.querySelector(".model-row-title strong");
    const meta = row.querySelector(".model-row-meta");
    if (title) {
      title.textContent = normalizedDisplayName(model);
    }
    if (meta) {
      meta.textContent = `${model.id}  •  ${deriveProviderBadge(model)}`;
    }
  }

  if (state.selectedModelUiId === model.uiId) {
    const inspectorTitle = elements.modelInspectorContent.querySelector(
      ".inspector-title h3",
    );
    const inspectorMeta =
      elements.modelInspectorContent.querySelector(".inspector-title p");
    if (inspectorTitle) {
      inspectorTitle.textContent = normalizedDisplayName(model);
    }
    if (inspectorMeta) {
      inspectorMeta.textContent = `${model.id}  •  ${deriveProviderBadge(model)}`;
    }
  }
}

function normalizedProviderBaseUrl(value) {
  return String(value || "")
    .trim()
    .replace(/\/+$/, "");
}

function findModelRowElement(uiId) {
  return (
    Array.from(document.querySelectorAll(".model-row")).find(
      (row) => row.dataset.uiId === uiId,
    ) ?? null
  );
}

function captureInspectorFocus(model) {
  if (!model) {
    return null;
  }

  const active = document.activeElement;
  if (!(active instanceof HTMLElement)) {
    return null;
  }

  if (!elements.modelInspectorContent.contains(active)) {
    return null;
  }

  const fieldKey = active.dataset?.fieldKey;
  if (!fieldKey) {
    return null;
  }

  const focus = {
    modelUiId: model.uiId,
    fieldKey,
    selectionStart: null,
    selectionEnd: null,
  };

  if (active instanceof HTMLInputElement) {
    try {
      focus.selectionStart = active.selectionStart;
      focus.selectionEnd = active.selectionEnd;
    } catch {
      // Some input types do not support selection ranges.
    }
  }

  return focus;
}

function restoreInspectorFocus(preservedFocus) {
  if (!preservedFocus) {
    return;
  }

  const selectedModel = state.models.find(
    (model) => model.uiId === preservedFocus.modelUiId,
  );
  if (!selectedModel || state.selectedModelUiId !== preservedFocus.modelUiId) {
    return;
  }

  const selector = `[data-field-key="${preservedFocus.fieldKey}"]`;
  const field = elements.modelInspectorContent.querySelector(selector);
  if (!(field instanceof HTMLElement)) {
    return;
  }

  field.focus();

  if (
    field instanceof HTMLInputElement &&
    preservedFocus.selectionStart !== null &&
    preservedFocus.selectionEnd !== null
  ) {
    try {
      field.setSelectionRange(
        preservedFocus.selectionStart,
        preservedFocus.selectionEnd,
      );
    } catch {
      // Some input types do not support selection ranges.
    }
  }
}
