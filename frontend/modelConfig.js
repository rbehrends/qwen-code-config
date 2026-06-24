import { elements, state } from "./shared.js";
import {
  cancelModelDrag,
  initializeModelReorder,
  moveModelWithinProtocol,
  removeModel,
  setDefaultModel,
  startModelDrag,
  syncModelDragStateWithModels,
} from "./modelReorder.js";
import {
  deriveProviderBadge,
  initializeModelInspector,
  normalizedDisplayName,
  renderModelInspector,
} from "./modelInspector.js";
import {
  initializeEnvConfig,
  nextEnvKey,
  renderEnvVars,
  serializeEnvVars,
} from "./envConfig.js";

let deps = null;

export function initializeModelConfig(nextDeps) {
  deps = nextDeps;

  initializeEnvConfig({
    markStateChanged: deps.markStateChanged,
  });
  initializeModelReorder({
    ensureSelectedModel,
    markStateChanged: deps.markStateChanged,
    renderCatalog: deps.renderCatalog,
    renderFastModelControls: deps.renderFastModelControls,
    renderModels,
  });
  initializeModelInspector({
    confirmAndRemoveModel: deps.confirmAndRemoveModel,
    markStateChanged: deps.markStateChanged,
    renderModels,
  });
}

export function renderModels() {
  ensureSelectedModel();
  elements.configuredModelCount.textContent = `${state.models.length} model${state.models.length === 1 ? "" : "s"}`;
  elements.modelsLayout.classList.toggle(
    "inspector-open",
    shouldShowInspector(),
  );
  renderModelGroup(
    "openai",
    elements.openaiModelList,
    elements.openaiModelCount,
  );
  renderModelGroup(
    "anthropic",
    elements.anthropicModelList,
    elements.anthropicModelCount,
  );
  renderModelInspector(getSelectedModel());
}

export function renderWarnings() {
  // Warning text is derived by the Rust preview path so the frontend does not
  // grow a parallel warning engine with drift-prone semantics.
  const warnings = state.warnings;

  if (warnings.length === 0) {
    elements.modelWarnings.hidden = true;
    elements.modelWarnings.replaceChildren();
    return;
  }

  elements.modelWarnings.hidden = false;
  elements.modelWarnings.replaceChildren();

  const list = document.createElement("ul");
  list.className = "warning-list";

  warnings.forEach((warning) => {
    const item = document.createElement("li");
    item.textContent = warning;
    list.append(item);
  });

  elements.modelWarnings.append(list);
}

export function serializeModels() {
  return state.models.map((model) => ({
    uiId: model.uiId,
    protocol: model.protocol,
    id: model.id,
    name: model.name,
    baseUrl: model.baseUrl,
    envKey: model.envKey,
    contextWindowSize: model.contextWindowSize,
    temperature: model.temperature,
    topP: model.topP,
    maxTokens: model.maxTokens,
    reasoningMode: model.reasoningMode ?? "default",
    reasoningEffort: model.reasoningEffort ?? null,
    reasoningBudgetTokens: model.reasoningBudgetTokens,
    samplingParams: model.samplingParams ?? {},
    extraBody: model.extraBody ?? {},
    rawModel: model.rawModel ?? {},
    isDefault: Boolean(model.isDefault),
    isDuplicate: Boolean(model.isDuplicate),
  }));
}

// This is only a local UX precheck for add buttons and inline validation.
// Rust preview/save remains authoritative for duplicate detection semantics.
export function hasPotentialDuplicateConfiguredModel(
  protocol,
  modelId,
  baseUrl,
) {
  const candidateId = String(modelId || "").trim();
  const candidateBaseUrl = normalizedProviderBaseUrl(baseUrl);

  return state.models.some(
    (model) =>
      model.protocol === protocol &&
      model.id.trim() === candidateId &&
      normalizedProviderBaseUrl(model.baseUrl) === candidateBaseUrl,
  );
}

export function getSelectedModel() {
  return (
    state.models.find((model) => model.uiId === state.selectedModelUiId) ?? null
  );
}

export function ensureSelectedModel() {
  if (state.models.length === 0) {
    state.selectedModelUiId = null;
    return;
  }

  if (!getSelectedModel()) {
    state.selectedModelUiId = state.models[0].uiId;
  }
}

export function isNarrowModelsLayout() {
  return window.matchMedia("(max-width: 1380px)").matches;
}

export {
  cancelModelDrag,
  nextEnvKey,
  removeModel,
  renderEnvVars,
  serializeEnvVars,
  syncModelDragStateWithModels,
};

function renderModelGroup(protocol, container, counter) {
  container.replaceChildren();

  const models = state.models.filter((model) => model.protocol === protocol);
  counter.textContent = `${models.length} model${models.length === 1 ? "" : "s"}`;

  if (models.length === 0) {
    const empty = document.createElement("div");
    empty.className = "model-empty";
    empty.textContent = "No configured models in this protocol bucket.";
    container.append(empty);
    return;
  }

  models.forEach((model) => {
    container.append(renderModelRow(model));
  });
}

function renderModelRow(model) {
  const card = document.createElement("article");
  card.className = "model-row";
  card.dataset.uiId = model.uiId;
  card.tabIndex = 0;
  card.role = "button";
  card.setAttribute(
    "aria-pressed",
    String(model.uiId === state.selectedModelUiId),
  );
  if (isModelDuplicate(model)) {
    card.classList.add("duplicate-card");
  }
  if (model.uiId === state.selectedModelUiId) {
    card.classList.add("selected-model-row");
  }
  card.addEventListener("click", (event) => {
    if (isModelRowInteractiveTarget(event.target)) {
      return;
    }
    state.selectedModelUiId = model.uiId;
    renderModels();
  });
  card.addEventListener("pointerdown", (event) => {
    if (event.button !== 0 || isModelRowInteractiveTarget(event.target)) {
      return;
    }

    startModelDrag(model.uiId);
  });
  card.addEventListener("keydown", (event) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      state.selectedModelUiId = model.uiId;
      renderModels();
    }
  });

  const header = document.createElement("div");
  header.className = "model-row-header";

  const titleBlock = document.createElement("div");
  titleBlock.className = "model-row-title";

  const title = document.createElement("strong");
  title.textContent = normalizedDisplayName(model);
  titleBlock.append(title);

  const badgeRow = document.createElement("div");
  badgeRow.className = "model-row-actions";

  if (isModelDuplicate(model)) {
    const duplicateBadge = document.createElement("span");
    duplicateBadge.className = "warning-chip";
    duplicateBadge.textContent = "Duplicate";
    badgeRow.append(duplicateBadge);
  }

  const defaultButton = document.createElement("button");
  defaultButton.type = "button";
  defaultButton.className = `default-radio-button ${model.isDefault ? "selected" : ""}`;
  defaultButton.title = model.isDefault
    ? "Default model"
    : "Set as default model";
  defaultButton.setAttribute("aria-label", defaultButton.title);
  defaultButton.setAttribute("aria-pressed", String(model.isDefault));

  const defaultDot = document.createElement("span");
  defaultDot.className = "default-radio-dot";
  defaultButton.append(defaultDot);
  defaultButton.addEventListener("click", (event) => {
    event.stopPropagation();
    setDefaultModel(model.uiId);
  });

  const upButton = document.createElement("button");
  upButton.type = "button";
  upButton.className = "secondary-button compact-button icon-button";
  upButton.textContent = "↑";
  upButton.title = "Move up";
  upButton.setAttribute("aria-label", upButton.title);
  upButton.addEventListener("click", (event) => {
    event.stopPropagation();
    moveModelWithinProtocol(model.uiId, -1);
  });

  const downButton = document.createElement("button");
  downButton.type = "button";
  downButton.className = "secondary-button compact-button icon-button";
  downButton.textContent = "↓";
  downButton.title = "Move down";
  downButton.setAttribute("aria-label", downButton.title);
  downButton.addEventListener("click", (event) => {
    event.stopPropagation();
    moveModelWithinProtocol(model.uiId, 1);
  });

  const editButton = document.createElement("button");
  editButton.type = "button";
  editButton.className = "secondary-button compact-button icon-button";
  editButton.textContent = "✎";
  editButton.title = "Edit";
  editButton.setAttribute("aria-label", editButton.title);
  editButton.addEventListener("click", (event) => {
    event.stopPropagation();
    state.selectedModelUiId = model.uiId;
    state.inspectorVisible = true;
    renderModels();
  });

  const removeButton = document.createElement("button");
  removeButton.type = "button";
  removeButton.className =
    "secondary-button compact-button icon-button danger-button";
  removeButton.textContent = "🗑";
  removeButton.title = "Remove";
  removeButton.setAttribute("aria-label", removeButton.title);
  removeButton.addEventListener("click", async (event) => {
    event.stopPropagation();
    await deps.confirmAndRemoveModel(model.uiId);
  });

  badgeRow.append(
    defaultButton,
    upButton,
    downButton,
    editButton,
    removeButton,
  );
  header.append(titleBlock, badgeRow);

  const subtitle = document.createElement("small");
  subtitle.className = "model-row-meta";
  subtitle.textContent = `${model.id}  •  ${deriveProviderBadge(model)}`;

  card.append(header, subtitle);
  return card;
}

function shouldShowInspector() {
  if (!getSelectedModel()) {
    return false;
  }

  return isNarrowModelsLayout() ? state.inspectorVisible : true;
}

function isModelDuplicate(model) {
  return Boolean(model.isDuplicate);
}

function isModelRowInteractiveTarget(target) {
  return (
    target instanceof Element &&
    Boolean(target.closest("button, input, select, textarea, a, label"))
  );
}

function normalizedProviderBaseUrl(value) {
  return String(value || "")
    .trim()
    .replace(/\/+$/, "");
}
