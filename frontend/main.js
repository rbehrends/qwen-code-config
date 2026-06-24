import {
  buildInitialProviderProfiles,
  customProviderProfilesForSave,
  getSelectedProviderProfile,
  initializeProviderWorkbench,
  renderCatalog,
  renderProviderProfiles,
  renderProviderWorkbench,
} from "./providerWorkbench.js";
import {
  DEFAULT_SETTINGS_PATH,
  IS_LINUX,
  elements,
  optionControls,
  state,
} from "./shared.js";
import {
  applyLayoutDensity,
  applyThemeMode,
  cloneJsonValue,
  disableTextAssistance,
  normalizeLayoutDensity,
  normalizeModelNameRewriteRules,
  normalizeThemeMode,
  parseSnapshotJson,
  prettifyModelName,
} from "./utils.js";
import {
  buildModelDraft as buildModelDraftApi,
  fetchProviderModels,
  getPendingOpenPath,
  getModelNameRewriteRules,
  hasTauriApi,
  loadSettingsSnapshot,
  loadUiStateSnapshot,
  previewSettings,
  saveOptions,
  saveOptionsAs,
  saveUiState,
  syncMenuUiState as syncMenuUiStateApi,
} from "./api.js";
import {
  chooseSavePath,
  closeCurrentWindow,
  confirmDialog,
  openSettingsPathDialog,
  registerAppMenuListener,
  registerOpenFileListener,
  registerWindowDropListener,
} from "./host.js";
import {
  cancelModelDrag,
  hasPotentialDuplicateConfiguredModel,
  initializeModelConfig,
  isNarrowModelsLayout,
  nextEnvKey,
  removeModel,
  renderEnvVars,
  renderModels,
  renderWarnings,
  serializeEnvVars,
  serializeModels,
  syncModelDragStateWithModels,
} from "./modelConfig.js";
import {
  buildFastModelRequest,
  initializeFastModel,
  normalizeFastModelSnapshot,
  renderFastModelControls,
} from "./fastModel.js";

[
  elements.pathInput,
  elements.providerLabelInput,
  elements.providerBaseUrlInput,
  elements.providerEnvKeyInput,
  elements.manualModelIdInput,
  elements.manualModelNameInput,
  elements.catalogFilterInput,
].forEach((input) => disableTextAssistance(input));

document.body.dataset.platform = IS_LINUX ? "linux" : "other";
applyLayoutDensity(state.layoutDensity);
applyThemeMode(state.themeMode);
elements.layoutDensitySelect.value = state.layoutDensity;
elements.themeModeSelect.value = state.themeMode;

document.querySelectorAll(".nav-item").forEach((button) => {
  button.addEventListener("click", () => {
    const section = button.dataset.section;

    document
      .querySelectorAll(".nav-item")
      .forEach((item) => item.classList.toggle("active", item === button));

    document
      .querySelectorAll(".view")
      .forEach((view) => (view.hidden = view.id !== `section-${section}`));
  });
});

optionControls.forEach(([key, control]) => {
  control.addEventListener("change", () => {
    state.options[key] = control.checked;
    markStateChanged();
  });
});

elements.layoutDensitySelect.addEventListener("change", () => {
  setLayoutDensity(elements.layoutDensitySelect.value);
});

elements.themeModeSelect.addEventListener("change", () => {
  setThemeMode(elements.themeModeSelect.value);
});

elements.closeInspectorButton.addEventListener("click", () => {
  state.inspectorVisible = false;
  renderModels();
});

elements.reloadButton.addEventListener("click", () => {
  void openRequestedPath(state.path);
});

elements.browsePathButton.addEventListener("click", async () => {
  try {
    const selected = await openSettingsPathDialog();
    if (typeof selected === "string") {
      elements.pathInput.value = selected;
      await openRequestedPath(selected);
    } else if (selected === undefined) {
      setStatus("File picker unavailable", "error");
    }
  } catch (error) {
    setStatus(error, "error");
  }
});

elements.loadPathButton.addEventListener("click", () => {
  void openRequestedPath(
    elements.pathInput.value.trim() || DEFAULT_SETTINGS_PATH,
  );
});

elements.addEnvButton.addEventListener("click", () => {
  state.envVars.push({
    key: nextEnvKey(),
    value: "",
    revealed: false,
  });
  renderEnvVars();
  markStateChanged();
});

elements.saveButton.addEventListener("click", async () => {
  await performSave();
});

elements.saveAsButton.addEventListener("click", async () => {
  await performSaveAs();
});

initializeModelConfig({
  markStateChanged,
  renderCatalog,
  renderFastModelControls,
  confirmAndRemoveModel,
});
initializeFastModel({
  markStateChanged,
});

initializeProviderWorkbench({
  queueUiStatePersist,
  setStatus,
  confirmRemoveCustomProvider,
  addConfiguredModelFromSource,
  hasPotentialDuplicateConfiguredModel,
  fetchProviderModels,
});

if (hasTauriApi()) {
  void initializeApp();
} else {
  setStatus("Tauri API unavailable", "error");
}

async function initializeApp() {
  registerOpenFileListener(async (path) => {
    elements.pathInput.value = path;
    await openRequestedPath(path);
  });

  registerAppMenuListener(async (command) => {
    switch (command) {
      case "save":
        await performSave();
        break;
      case "save-as":
        await performSaveAs();
        break;
      case "reload":
        await openRequestedPath(state.path);
        break;
      case "close-window":
        await requestQuit();
        break;
      case "theme-system":
        setThemeMode("system");
        break;
      case "theme-light":
        setThemeMode("light");
        break;
      case "theme-dark":
        setThemeMode("dark");
        break;
      case "density-compact":
        setLayoutDensity("compact");
        break;
      case "density-comfortable":
        setLayoutDensity("comfortable");
        break;
      case "density-spacious":
        setLayoutDensity("spacious");
        break;
      case "quit":
        await requestQuit();
        break;
      default:
        break;
    }
  });

  registerWindowDropListener(async (path) => {
    elements.pathInput.value = path;
    await openRequestedPath(path);
  });

  await loadModelNameFormatting();
  await loadUiState();

  try {
    const pending = await getPendingOpenPath();
    const initialPath = pending?.path || state.path;
    await loadSettings(initialPath);
  } catch {
    await loadSettings(state.path);
  }
}

async function loadModelNameFormatting() {
  try {
    const rules = await getModelNameRewriteRules();
    state.modelNameRewriteRules = normalizeModelNameRewriteRules(rules);
  } catch {
    state.modelNameRewriteRules = [];
  }
}

async function openRequestedPath(path) {
  if (hasUnsavedChanges() && !(await confirmReplaceCurrentSettings(path))) {
    elements.pathInput.value = state.path;
    return;
  }

  await loadSettings(path);
}

async function loadSettings(path) {
  setStatus("Loading", "");

  try {
    const snapshot = await loadSettingsSnapshot(path);
    await applySnapshot(snapshot);
    setStatus("Valid", "ready");
  } catch (error) {
    setStatus(error, "error");
  }
}

function buildSettingsSaveRequest(overrides = {}) {
  const fastModel = buildFastModelRequest();

  return {
    ...overrides,
    options: state.options,
    envVars: serializeEnvVars(),
    models: serializeModels(),
    ...(fastModel ? { fastModel } : {}),
  };
}

async function saveSettingsWithCommand(command, overrides = {}) {
  setStatus("Saving", "");
  const requestChangeSerial = state.changeSerial;
  const saveRequestId = state.saveRequestId + 1;
  state.saveRequestId = saveRequestId;

  try {
    const request = buildSettingsSaveRequest(overrides);
    const snapshot =
      command === "save_options"
        ? await saveOptions(request)
        : await saveOptionsAs(request);
    if (saveRequestId !== state.saveRequestId) {
      return;
    }

    if (state.changeSerial !== requestChangeSerial) {
      await applySavedBaseSnapshot(snapshot);
      setStatus("Saved; newer edits not yet saved", "dirty");
      return;
    }

    await applySnapshot(snapshot);
    setStatusAfterSave(false);
  } catch (error) {
    if (saveRequestId === state.saveRequestId) {
      setStatus(error, "error");
    }
  }
}

async function performSave() {
  if (!hasUnsavedChanges()) {
    return;
  }

  await saveSettingsWithCommand("save_options", {
    path: state.path,
  });
}

async function performSaveAs() {
  try {
    const selected = await chooseSavePath(state.path);
    if (typeof selected !== "string") {
      if (selected === undefined) {
        setStatus("Save dialog unavailable", "error");
      }
      return;
    }

    await saveSettingsWithCommand("save_options_as", {
      sourcePath: state.path,
      targetPath: selected,
    });
  } catch (error) {
    setStatus(error, "error");
  }
}

async function requestQuit() {
  if (hasUnsavedChanges() && !(await confirmQuitWithUnsavedChanges())) {
    return;
  }

  try {
    await closeCurrentWindow();
  } catch (error) {
    setStatus(error, "error");
  }
}

async function loadUiState() {
  try {
    const snapshot = await loadUiStateSnapshot();
    applyUiStateSnapshot(snapshot);
  } catch (error) {
    setStatus(error, "error");
  }
}

async function confirmAction(message, options) {
  return confirmDialog(message, options);
}

async function confirmReplaceCurrentSettings(path) {
  const targetPath = String(path || "").trim() || DEFAULT_SETTINGS_PATH;
  return confirmAction(`Discard unsaved changes and open ${targetPath}?`, {
    title: "Discard Unsaved Changes",
    okLabel: "Discard",
  });
}

async function confirmQuitWithUnsavedChanges() {
  return confirmAction("Discard unsaved changes and quit?", {
    title: "Discard Unsaved Changes",
    okLabel: "Quit",
  });
}

async function confirmRemoveModel(uiId) {
  const model = state.models.find((entry) => entry.uiId === uiId);
  if (!model) {
    return false;
  }

  return confirmAction(`Remove model ${normalizedDisplayName(model)}?`, {
    title: "Remove Model",
    okLabel: "Remove",
  });
}

async function confirmRemoveCustomProvider(profileId) {
  const profile = state.providerProfiles.find(
    (entry) => entry.profileId === profileId && !entry.presetId,
  );
  if (!profile) {
    return false;
  }

  const label = profile.label?.trim() || "Untitled provider";
  return confirmAction(`Remove custom provider ${label}?`, {
    title: "Remove Custom Provider",
    okLabel: "Remove",
  });
}

async function confirmAndRemoveModel(uiId) {
  if (await confirmRemoveModel(uiId)) {
    removeModel(uiId);
  }
}

function hasUnsavedChanges() {
  return state.dirty || state.previewCanonicalJson !== state.baseCanonicalJson;
}

async function addConfiguredModelFromSource(profile, source) {
  const model = await buildModelDraftApi(
    profile,
    source,
    state.models.some((entry) => entry.isDefault),
  );
  state.models.push(model);
  state.selectedModelUiId = state.models.at(-1)?.uiId ?? null;
  state.inspectorVisible = !isNarrowModelsLayout();
  renderFastModelControls();
  renderModels();
  renderCatalog();
  markStateChanged();
}

async function applySnapshot(snapshot) {
  cancelModelDrag();
  state.path = snapshot.path;
  state.options = snapshot.options;
  state.envVars = snapshot.envVars.map((envVar) => ({
    ...envVar,
    revealed: false,
  }));
  state.models = snapshot.models.map((model) => ({
    ...model,
  }));
  state.fastModel = normalizeFastModelSnapshot(snapshot.fastModel);
  state.fastModelTouched = false;
  state.warnings = Array.isArray(snapshot.warnings) ? snapshot.warnings : [];
  state.lastBackupPath = snapshot.lastBackupPath ?? null;
  state.statusDetail = null;
  state.providerPresets = Array.isArray(snapshot.providers)
    ? snapshot.providers
    : [];
  state.baseJsonObject = parseSnapshotJson(snapshot.json);
  state.baseCanonicalJson = String(snapshot.json ?? "{}");
  state.previewCanonicalJson = state.baseCanonicalJson;
  state.previewJson = state.baseCanonicalJson;

  if (state.providerProfiles.length === 0) {
    state.providerProfiles = buildInitialProviderProfiles();
  }

  if (!state.selectedProviderProfileId || !getSelectedProviderProfile()) {
    state.selectedProviderProfileId =
      state.providerProfiles[0]?.profileId ?? null;
  }

  elements.currentPath.textContent = snapshot.path;
  elements.pathInput.value = snapshot.path;

  optionControls.forEach(([key, control]) => {
    control.checked = Boolean(state.options[key]);
  });

  renderEnvVars();
  renderProviderProfiles();
  renderProviderWorkbench();
  renderFastModelControls();
  renderModels();
  renderWarnings();
  renderHeaderNote();
  await refreshDerivedState();
}

function applyUiStateSnapshot(snapshot) {
  // Rust snapshots are already normalized through typed enums, so the frontend
  // should treat these persisted values as authoritative rather than re-derive them.
  state.layoutDensity = snapshot?.layoutDensity ?? "compact";
  state.themeMode = snapshot?.themeMode ?? "system";
  state.customProviderProfiles = Array.isArray(snapshot?.customProviderProfiles)
    ? snapshot.customProviderProfiles
    : [];
  applyLayoutDensity(state.layoutDensity);
  applyThemeMode(state.themeMode);
  elements.layoutDensitySelect.value = state.layoutDensity;
  elements.themeModeSelect.value = state.themeMode;
  void syncMenuUiState();
}

function setLayoutDensity(value) {
  const normalized = normalizeLayoutDensity(value);
  state.layoutDensity = normalized;
  applyLayoutDensity(normalized);
  elements.layoutDensitySelect.value = normalized;
  void syncMenuUiState();
  queueUiStatePersist();
}

function setThemeMode(value) {
  const normalized = normalizeThemeMode(value);
  state.themeMode = normalized;
  applyThemeMode(normalized);
  elements.themeModeSelect.value = normalized;
  void syncMenuUiState();
  queueUiStatePersist();
}

async function syncMenuUiState() {
  try {
    await syncMenuUiStateApi({
      canSave: hasUnsavedChanges(),
      layoutDensity: state.layoutDensity,
      themeMode: state.themeMode,
    });
  } catch (error) {
    setStatus(error, "error");
  }
}

function queueUiStatePersist() {
  if (state.persistUiStateTimer) {
    window.clearTimeout(state.persistUiStateTimer);
  }

  state.persistUiStateTimer = window.setTimeout(() => {
    state.persistUiStateTimer = null;
    void persistUiState();
  }, 200);
}

async function persistUiState() {
  try {
    const snapshot = await saveUiState({
      layoutDensity: state.layoutDensity,
      themeMode: state.themeMode,
      customProviderProfiles: customProviderProfilesForSave(),
    });
    state.layoutDensity = snapshot?.layoutDensity ?? "compact";
    state.themeMode = snapshot?.themeMode ?? "system";
    state.customProviderProfiles = Array.isArray(
      snapshot?.customProviderProfiles,
    )
      ? snapshot.customProviderProfiles
      : [];
    applyLayoutDensity(state.layoutDensity);
    applyThemeMode(state.themeMode);
    elements.layoutDensitySelect.value = state.layoutDensity;
    elements.themeModeSelect.value = state.themeMode;
  } catch (error) {
    setStatus(error, "error");
  }
}

function normalizedDisplayName(model) {
  const trimmed = String(model.name || "").trim();
  return trimmed || prettifyModelName(model.id);
}

function setDirty(dirty) {
  state.dirty = dirty;
  elements.saveButton.disabled = !dirty;
  void syncMenuUiState();

  if (dirty) {
    setStatus("Unsaved changes", "dirty");
  }
}

function setStatusAfterSave(hasPendingEdits) {
  if (hasPendingEdits) {
    setStatus("Saved; newer edits not yet saved", "dirty");
    return;
  }

  const warningCount = state.warnings.length;
  if (warningCount > 0) {
    setStatus("Saved with warnings", "dirty");
    return;
  }

  setStatus("Saved", "ready");
}

function renderHeaderNote() {
  const notes = [];

  if (state.statusDetail) {
    notes.push({
      text: state.statusDetail,
      className: "header-note-line error-note",
    });
  }

  if (state.lastBackupPath) {
    notes.push({
      text: `Backup: ${state.lastBackupPath}`,
      className: "header-note-line",
    });
  }

  if (notes.length === 0) {
    elements.backupPathNote.hidden = true;
    elements.backupPathNote.replaceChildren();
    return;
  }

  elements.backupPathNote.hidden = false;
  elements.backupPathNote.replaceChildren(
    ...notes.map(({ text, className }) => {
      const line = document.createElement("div");
      line.className = className;
      line.textContent = text;
      return line;
    }),
  );
}

function setStatus(message, type) {
  const detail = String(message);
  if (type === "error") {
    elements.statusPill.textContent = "Error";
    state.statusDetail = detail;
  } else {
    elements.statusPill.textContent = detail;
    state.statusDetail = null;
  }
  elements.statusPill.className = `status-pill ${type}`.trim();
  renderHeaderNote();
}

function renderJsonPreview() {
  elements.jsonPreview.textContent = state.previewJson;
}

function markStateChanged() {
  state.changeSerial += 1;
  setDirty(true);
  void refreshDerivedState();
}

async function applySavedBaseSnapshot(snapshot) {
  state.path = snapshot.path;
  state.lastBackupPath = snapshot.lastBackupPath ?? null;
  state.statusDetail = null;
  state.providerPresets = Array.isArray(snapshot.providers)
    ? snapshot.providers
    : [];
  state.baseJsonObject = parseSnapshotJson(snapshot.json);
  state.baseCanonicalJson = String(snapshot.json ?? "{}");

  elements.currentPath.textContent = snapshot.path;
  elements.pathInput.value = snapshot.path;

  renderProviderProfiles();
  renderProviderWorkbench();
  renderHeaderNote();
  await refreshDerivedState();
}

async function refreshDerivedState() {
  const requestId = state.previewRequestId + 1;
  state.previewRequestId = requestId;

  try {
    const fastModel = buildFastModelRequest();
    const result = await previewSettings({
      baseJson: cloneJsonValue(state.baseJsonObject),
      options: state.options,
      envVars: serializeEnvVars(),
      models: serializeModels(),
      ...(fastModel ? { fastModel } : {}),
    });

    if (requestId !== state.previewRequestId) {
      return;
    }

    state.previewCanonicalJson = String(result.canonicalJson ?? "{}");
    state.previewJson = String(result.previewJson ?? "{}");
    state.warnings = Array.isArray(result.warnings) ? result.warnings : [];
    state.fastModel = normalizeFastModelSnapshot(result.fastModel);
    if (Array.isArray(result.models)) {
      const currentByUiId = new Map(
        state.models.map((model) => [model.uiId, model]),
      );
      // Preview is the semantic authority for normalized model state.
      // Frontend-only fields are merged back in, but duplicate/default flags
      // and other derived values come from Rust.
      state.models = result.models.map((model) => ({
        ...(currentByUiId.get(model.uiId) ?? {}),
        ...model,
      }));
      syncModelDragStateWithModels();
    }
    setDirty(state.previewCanonicalJson !== state.baseCanonicalJson);
    renderWarnings();
    renderFastModelControls();
    renderModels();
    renderJsonPreview();

    if (!state.dirty && elements.statusPill.classList.contains("dirty")) {
      setStatus("Valid", "ready");
    }
  } catch (error) {
    if (requestId === state.previewRequestId) {
      elements.jsonPreview.textContent = `Preview unavailable: ${error}`;
    }
  }
}
