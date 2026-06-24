const tauriCore = window.__TAURI__?.core;

export const invoke = tauriCore?.invoke;
export const openDialog = window.__TAURI__?.dialog?.open;
export const saveDialog = window.__TAURI__?.dialog?.save;
export const askDialog = window.__TAURI__?.dialog?.ask;
export const tauriEvent = window.__TAURI__?.event;
export const tauriWindow = window.__TAURI__?.window;
export const tauriWebviewWindow = window.__TAURI__?.webviewWindow;

export const SETTINGS_FILE_OPENED_EVENT = "settings-file-opened";
export const APP_MENU_COMMAND_EVENT = "app-menu-command";
export const DEFAULT_SETTINGS_PATH = "~/.qwen/settings.json";
export const IS_LINUX =
  navigator.userAgent.includes("Linux") &&
  !navigator.userAgent.includes("Mac OS X");

export const state = {
  path: DEFAULT_SETTINGS_PATH,
  options: {
    usageStatisticsEnabled: true,
    telemetryEnabled: false,
    enableAutoUpdate: true,
  },
  envVars: [],
  models: [],
  fastModel: {
    mode: "inherit",
    protocol: null,
    modelId: null,
    rawValue: null,
  },
  fastModelTouched: false,
  warnings: [],
  lastBackupPath: null,
  statusDetail: null,
  baseJsonObject: {},
  baseCanonicalJson: "{}",
  previewCanonicalJson: "{}",
  previewJson: "{}",
  providerPresets: [],
  providerProfiles: [],
  customProviderProfiles: [],
  selectedProviderProfileId: null,
  selectedModelUiId: null,
  inspectorVisible: false,
  draggedModelUiId: null,
  dragTargetModelUiId: null,
  dragDropPosition: null,
  changeSerial: 0,
  previewRequestId: 0,
  saveRequestId: 0,
  persistUiStateTimer: null,
  layoutDensity: "compact",
  themeMode: "system",
  dirty: false,
  modelNameRewriteRules: [],
};

export const elements = {
  currentPath: document.querySelector("#current-path"),
  pathInput: document.querySelector("#path-input"),
  statusPill: document.querySelector("#status-pill"),
  backupPathNote: document.querySelector("#backup-path-note"),
  saveButton: document.querySelector("#save-button"),
  saveAsButton: document.querySelector("#save-as-button"),
  reloadButton: document.querySelector("#reload-button"),
  browsePathButton: document.querySelector("#browse-path-button"),
  loadPathButton: document.querySelector("#load-path-button"),
  addEnvButton: document.querySelector("#add-env-button"),
  envList: document.querySelector("#env-list"),
  jsonPreview: document.querySelector("#json-preview"),
  layoutDensitySelect: document.querySelector("#layout-density-select"),
  themeModeSelect: document.querySelector("#theme-mode-select"),
  usageStatisticsToggle: document.querySelector("#usage-statistics-toggle"),
  telemetryToggle: document.querySelector("#telemetry-toggle"),
  autoUpdateToggle: document.querySelector("#auto-update-toggle"),
  modelsLayout: document.querySelector("#models-layout"),
  modelWarnings: document.querySelector("#model-warnings"),
  configuredModelCount: document.querySelector("#configured-model-count"),
  providerSelect: document.querySelector("#provider-select"),
  fastModelSelectGroup: document.querySelector("#fast-model-select-group"),
  fastModelSelect: document.querySelector("#fast-model-select"),
  fastModelNote: document.querySelector("#fast-model-note"),
  providerLabelInput: document.querySelector("#provider-label-input"),
  providerProtocolSelect: document.querySelector("#provider-protocol-select"),
  providerBaseUrlInput: document.querySelector("#provider-base-url-input"),
  providerEnvKeyInput: document.querySelector("#provider-env-key-input"),
  newProviderButton: document.querySelector("#new-provider-button"),
  removeProviderButton: document.querySelector("#remove-provider-button"),
  fetchProviderModelsButton: document.querySelector(
    "#fetch-provider-models-button",
  ),
  manualModelIdInput: document.querySelector("#manual-model-id-input"),
  manualModelNameInput: document.querySelector("#manual-model-name-input"),
  addManualModelButton: document.querySelector("#add-manual-model-button"),
  catalogSummary: document.querySelector("#catalog-summary"),
  catalogFilterInput: document.querySelector("#catalog-filter-input"),
  catalogFilterMode: document.querySelector("#catalog-filter-mode"),
  catalogList: document.querySelector("#catalog-list"),
  openaiModelCount: document.querySelector("#openai-model-count"),
  anthropicModelCount: document.querySelector("#anthropic-model-count"),
  openaiModelList: document.querySelector("#openai-model-list"),
  anthropicModelList: document.querySelector("#anthropic-model-list"),
  modelInspectorPanel: document.querySelector("#model-inspector-panel"),
  modelInspectorContent: document.querySelector("#model-inspector-content"),
  closeInspectorButton: document.querySelector("#close-inspector-button"),
};

export const optionControls = [
  ["usageStatisticsEnabled", elements.usageStatisticsToggle],
  ["telemetryEnabled", elements.telemetryToggle],
  ["enableAutoUpdate", elements.autoUpdateToggle],
];
