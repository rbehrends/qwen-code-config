import { elements, state } from "./shared.js";
import { openPathDialog } from "./host.js";
import { disableTextAssistance } from "./utils.js";

let deps = null;

export function initializeMcpConfig(nextDeps) {
  deps = nextDeps;

  elements.addMcpServerButton.addEventListener("click", () => {
    const server = createEmptyServer();
    state.mcpServers.push(server);
    state.selectedMcpServerUiId = server.uiId;
    renderMcpServers();
    deps.markStateChanged();
  });

  elements.removeMcpServerButton.addEventListener("click", async () => {
    const server = getSelectedMcpServer();
    if (!server) {
      return;
    }

    await deps.confirmAndRemoveMcpServer(server.uiId);
  });
}

export function renderMcpServers() {
  ensureSelectedMcpServer();
  renderMcpServerList();
  renderMcpInspector();
}

export function renderMcpWarnings() {
  const warnings = state.warnings.filter((warning) => warning.startsWith("MCP:"));

  if (warnings.length === 0) {
    elements.mcpWarnings.hidden = true;
    elements.mcpWarnings.replaceChildren();
    return;
  }

  elements.mcpWarnings.hidden = false;
  elements.mcpWarnings.replaceChildren();

  const list = document.createElement("ul");
  list.className = "warning-list";

  warnings.forEach((warning) => {
    const item = document.createElement("li");
    item.textContent = warning.replace(/^MCP:\s*/, "");
    list.append(item);
  });

  elements.mcpWarnings.append(list);
}

export function serializeMcpServers() {
  return state.mcpServers.map((server) => ({
    uiId: server.uiId,
    name: server.name,
    enabled: Boolean(server.enabled),
    transport: server.transport,
    command: server.command,
    args: Array.isArray(server.args) ? server.args : [],
    cwd: server.cwd,
    envVars: Array.isArray(server.envVars) ? server.envVars : [],
    url: server.url,
    headers: Array.isArray(server.headers) ? server.headers : [],
    timeout: server.timeout,
    rawServer: server.rawServer ?? {},
  }));
}

export function removeMcpServer(uiId) {
  state.mcpServers = state.mcpServers.filter((server) => server.uiId !== uiId);
  if (state.selectedMcpServerUiId === uiId) {
    state.selectedMcpServerUiId = state.mcpServers[0]?.uiId ?? null;
  }
  renderMcpServers();
  deps.markStateChanged();
}

export function normalizeMcpServersSnapshot(servers) {
  if (!Array.isArray(servers)) {
    return [];
  }

  return servers.map((server, index) => ({
    uiId: String(server?.uiId || `saved-mcp-${index}`),
    name: String(server?.name || ""),
    enabled: server?.enabled !== false,
    transport: normalizeTransport(server?.transport),
    command: String(server?.command || ""),
    args: Array.isArray(server?.args)
      ? server.args.map((entry) => String(entry ?? ""))
      : [],
    cwd: String(server?.cwd || ""),
    envVars: normalizeKeyValueEntries(server?.envVars),
    url: String(server?.url || ""),
    headers: normalizeKeyValueEntries(server?.headers),
    timeout:
      Number.isInteger(server?.timeout) && Number(server.timeout) >= 0
        ? Number(server.timeout)
        : null,
    rawServer:
      server?.rawServer && typeof server.rawServer === "object"
        ? server.rawServer
        : {},
  }));
}

function renderMcpServerList() {
  elements.mcpServerCount.textContent = `${state.mcpServers.length} server${state.mcpServers.length === 1 ? "" : "s"}`;
  elements.mcpServerList.replaceChildren();

  if (state.mcpServers.length === 0) {
    const empty = document.createElement("div");
    empty.className = "model-empty";
    empty.textContent = "No configured MCP servers.";
    elements.mcpServerList.append(empty);
    return;
  }

  state.mcpServers.forEach((server) => {
    elements.mcpServerList.append(renderMcpServerRow(server));
  });
}

function renderMcpServerRow(server) {
  const row = document.createElement("article");
  row.className = "model-row mcp-server-row";
  row.dataset.uiId = server.uiId;
  if (server.uiId === state.selectedMcpServerUiId) {
    row.classList.add("selected-model-row");
  }
  row.tabIndex = 0;
  row.role = "button";
  row.setAttribute(
    "aria-pressed",
    String(server.uiId === state.selectedMcpServerUiId),
  );

  row.addEventListener("click", () => {
    state.selectedMcpServerUiId = server.uiId;
    renderMcpServers();
  });

  row.addEventListener("keydown", (event) => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      state.selectedMcpServerUiId = server.uiId;
      renderMcpServers();
    }
  });

  const header = document.createElement("div");
  header.className = "model-row-header";

  const titleBlock = document.createElement("div");
  titleBlock.className = "model-row-title";

  const title = document.createElement("strong");
  title.textContent = server.name.trim() || "Unnamed server";
  titleBlock.append(title);

  const badge = document.createElement("span");
  badge.className = "warning-chip";
  badge.textContent = transportLabel(server.transport);

  if (!server.enabled) {
    row.classList.add("mcp-server-row-disabled");
  }

  const enabledButton = document.createElement("button");
  enabledButton.type = "button";
  enabledButton.className = `secondary-button compact-button ${server.enabled ? "" : "selected-button"}`.trim();
  enabledButton.textContent = server.enabled ? "Disable" : "Enable";
  enabledButton.addEventListener("click", (event) => {
    event.stopPropagation();
    server.enabled = !server.enabled;
    renderMcpServers();
    deps.markStateChanged();
  });

  const actions = document.createElement("div");
  actions.className = "model-row-actions";

  const editButton = document.createElement("button");
  editButton.type = "button";
  editButton.className = "secondary-button compact-button";
  editButton.textContent = "Edit";
  editButton.addEventListener("click", (event) => {
    event.stopPropagation();
    state.selectedMcpServerUiId = server.uiId;
    renderMcpServers();
    focusMcpInspector();
  });

  const removeButton = document.createElement("button");
  removeButton.type = "button";
  removeButton.className =
    "secondary-button compact-button danger-button";
  removeButton.textContent = "Remove";
  removeButton.addEventListener("click", async (event) => {
    event.stopPropagation();
    await deps.confirmAndRemoveMcpServer(server.uiId);
  });

  actions.append(badge, enabledButton, editButton, removeButton);

  header.append(titleBlock, actions);

  const subtitle = document.createElement("small");
  subtitle.className = "model-row-meta";
  subtitle.textContent = buildServerSubtitle(server);

  row.append(header, subtitle);
  return row;
}

function renderMcpInspector() {
  const server = getSelectedMcpServer();
  const preservedFocus = captureMcpInspectorFocus(server);
  elements.removeMcpServerButton.disabled = !server;
  elements.mcpInspectorContent.replaceChildren();

  if (!server) {
    const empty = document.createElement("div");
    empty.className = "model-inspector-empty";
    empty.textContent = "Select a server to edit it.";
    elements.mcpInspectorContent.append(empty);
    return;
  }

  const form = document.createElement("div");
  form.className = "mcp-inspector-form";

  const nameField = renderTextField(
    "Name",
    server.name,
    (value) => {
      server.name = value;
      syncMcpPresentation(server);
    },
    { fieldKey: "name" },
  );
  const transportField = renderSelectField(
    "Transport",
    server.transport,
    [
      ["stdio", "Stdio"],
      ["http", "HTTP"],
      ["sse", "SSE"],
    ],
    (value) => {
      server.transport = normalizeTransport(value);
      syncMcpPresentation(server);
    },
    { fieldKey: "transport" },
  );
  const timeoutField = renderNumberField(
    "Timeout (ms)",
    server.timeout,
    (value) => {
      server.timeout = value;
    },
    { fieldKey: "timeout" },
  );

  nameField.classList.add("mcp-field-span");
  const enabledField = renderToggleField("Enabled", server.enabled, (value) => {
    server.enabled = value;
    syncMcpPresentation(server);
    renderMcpServers();
  });
  timeoutField.classList.add("mcp-field-span");

  form.append(
    nameField,
    enabledField,
    transportField,
  );

  if (server.transport === "stdio") {
    const commandField = renderTextField(
      "Command",
      server.command,
      (value) => {
        server.command = value;
        syncMcpPresentation(server);
      },
      {
        fieldKey: "command",
        actionLabel: "Browse",
        onAction: async () => {
          const selected = await browseForCommandPath(server.command);
          if (typeof selected !== "string") {
            return;
          }

          server.command = selected;
          syncMcpPresentation(server);
          renderMcpInspector();
          deps.markStateChanged();
        },
      },
    );
    commandField.classList.add("mcp-field-span");

    const argsField = renderStringListField(
      "Arguments",
      () => server.args,
      (next) => {
        server.args = next;
      },
    );
    const cwdField = renderTextField(
      "Working Directory",
      server.cwd,
      (value) => {
        server.cwd = value;
      },
      {
        fieldKey: "cwd",
        actionLabel: "Browse",
        onAction: async () => {
          const selected = await browseForDirectoryPath(server.cwd);
          if (typeof selected !== "string") {
            return;
          }

          server.cwd = selected;
          renderMcpInspector();
          deps.markStateChanged();
        },
      },
    );
    const envField = renderKeyValueField(
      "Environment",
      () => server.envVars,
      (next) => {
        server.envVars = next;
      },
    );

    argsField.classList.add("mcp-field-span");
    cwdField.classList.add("mcp-field-span");
    envField.classList.add("mcp-field-span");

    form.append(
      commandField,
      cwdField,
      argsField,
      envField,
    );
  } else {
    const urlField = renderTextField(
      server.transport === "http" ? "HTTP URL" : "SSE URL",
      server.url,
      (value) => {
        server.url = value;
        syncMcpPresentation(server);
      },
      { fieldKey: "url" },
    );
    const headersField = renderKeyValueField(
      "Headers",
      () => server.headers,
      (next) => {
        server.headers = next;
      },
    );

    urlField.classList.add("mcp-field-span");
    headersField.classList.add("mcp-field-span");

    form.append(
      urlField,
      headersField,
    );
  }

  form.append(timeoutField);

  elements.mcpInspectorContent.append(form);
  restoreMcpInspectorFocus(preservedFocus);
}

function renderTextField(label, value, onChange, options = {}) {
  const field = document.createElement("label");
  field.className = "mcp-field";

  const title = document.createElement("span");
  title.className = "field-label";
  title.textContent = label;

  const controlRow = document.createElement("div");
  controlRow.className = options.onAction
    ? "mcp-field-control-row"
    : "mcp-field-control-row mcp-field-control-row-single";

  const input = document.createElement("input");
  input.className = "field-input";
  input.type = "text";
  input.value = value;
  if (label === "Name") {
    input.dataset.mcpPrimaryField = "true";
  }
  if (options.fieldKey) {
    input.dataset.fieldKey = options.fieldKey;
  }
  disableTextAssistance(input);
  input.addEventListener("input", () => {
    onChange(input.value);
    deps.markStateChanged();
  });

  controlRow.append(input);

  if (options.onAction) {
    const actionButton = document.createElement("button");
    actionButton.type = "button";
    actionButton.className = "secondary-button compact-button";
    actionButton.textContent = options.actionLabel || "Browse";
    actionButton.addEventListener("click", async () => {
      await options.onAction();
    });
    controlRow.append(actionButton);
  }

  field.append(title, controlRow);
  return field;
}

function renderNumberField(label, value, onChange, options = {}) {
  const field = document.createElement("label");
  field.className = "mcp-field";

  const title = document.createElement("span");
  title.className = "field-label";
  title.textContent = label;

  const input = document.createElement("input");
  input.className = "field-input";
  input.type = "number";
  input.min = "0";
  input.step = "1";
  input.value = Number.isInteger(value) ? String(value) : "";
  if (options.fieldKey) {
    input.dataset.fieldKey = options.fieldKey;
  }
  input.addEventListener("input", () => {
    const trimmed = input.value.trim();
    onChange(trimmed ? Number.parseInt(trimmed, 10) : null);
    deps.markStateChanged();
  });

  field.append(title, input);
  return field;
}

function renderToggleField(label, value, onChange) {
  const field = document.createElement("label");
  field.className = "mcp-field";

  const title = document.createElement("span");
  title.className = "field-label";
  title.textContent = label;

  const input = document.createElement("input");
  input.type = "checkbox";
  input.checked = Boolean(value);
  input.addEventListener("change", () => {
    onChange(input.checked);
    deps.markStateChanged();
  });

  field.append(title, input);
  return field;
}

function renderSelectField(label, value, options, onChange, selectOptions = {}) {
  const field = document.createElement("label");
  field.className = "mcp-field";

  const title = document.createElement("span");
  title.className = "field-label";
  title.textContent = label;

  const select = document.createElement("select");
  select.className = "field-input";
  if (selectOptions.fieldKey) {
    select.dataset.fieldKey = selectOptions.fieldKey;
  }

  options.forEach(([optionValue, optionLabel]) => {
    const option = document.createElement("option");
    option.value = optionValue;
    option.textContent = optionLabel;
    select.append(option);
  });

  select.value = value;
  select.addEventListener("change", () => {
    onChange(select.value);
    renderMcpServers();
    deps.markStateChanged();
  });

  field.append(title, select);
  return field;
}

function renderStringListField(label, getValues, onChange) {
  const field = document.createElement("div");
  field.className = "mcp-field";

  const list = document.createElement("div");
  list.className = "mcp-sublist";
  const listPrefix = label.toLowerCase();

  const emptyState = renderEmptySublist("No values.");

  const syncEmptyState = () => {
    const hasRows = list.querySelector(".mcp-string-row");
    if (hasRows) {
      emptyState.remove();
      return;
    }

    if (!list.contains(emptyState)) {
      list.append(emptyState);
    }
  };

  const renumberRows = () => {
    Array.from(list.querySelectorAll(".mcp-string-row")).forEach((row, index) => {
      row.dataset.index = String(index);
      const input = row.querySelector("input");
      if (input) {
        input.dataset.fieldKey = `${listPrefix}-${index}`;
      }
    });
  };

  const createRow = (value = "") => {
    const row = document.createElement("div");
    row.className = "env-row mcp-string-row";

    const input = document.createElement("input");
    input.className = "field-input";
    input.type = "text";
    input.value = value;
    disableTextAssistance(input);
    input.addEventListener("input", () => {
      const index = Number(row.dataset.index);
      const next = [...(getValues() || [])];
      next[index] = input.value;
      onChange(next);
      deps.markStateChanged();
    });
    input.addEventListener("keydown", (event) => {
      if (event.key !== "Enter") {
        return;
      }

      event.preventDefault();
      const index = Number(row.dataset.index);
      const next = [...(getValues() || [])];
      next.splice(index + 1, 0, "");
      onChange(next);

      const newRow = createRow("");
      row.after(newRow);
      renumberRows();
      syncEmptyState();
      deps.markStateChanged();
      focusNearestMcpListField(`${listPrefix}-${index + 1}`);
    });

    const removeButton = document.createElement("button");
    removeButton.type = "button";
    removeButton.className = "secondary-button compact-button danger-button";
    removeButton.textContent = "Remove";
    removeButton.addEventListener("click", () => {
      const index = Number(row.dataset.index);
      const next = (getValues() || []).filter((_, itemIndex) => itemIndex !== index);
      onChange(next);

      const focusTarget =
        row.nextElementSibling?.querySelector("input") ??
        row.previousElementSibling?.querySelector("input");

      row.remove();
      renumberRows();
      syncEmptyState();
      deps.markStateChanged();

      if (focusTarget instanceof HTMLElement) {
        focusTarget.focus({ preventScroll: true });
      }
    });

    row.append(input, removeButton);
    return row;
  };

  const header = renderListHeader(label, "Add", () => {
    const next = [...(getValues() || []), ""];
    onChange(next);
    list.append(createRow(""));
    renumberRows();
    syncEmptyState();
    deps.markStateChanged();
    focusNearestMcpListField(`${listPrefix}-${next.length - 1}`);
  });

  const values = getValues() || [];
  values.forEach((value) => {
    list.append(createRow(value));
  });
  renumberRows();
  syncEmptyState();

  field.append(header, list);
  return field;
}

function renderKeyValueField(label, getEntries, onChange) {
  const field = document.createElement("div");
  field.className = "mcp-field";

  const list = document.createElement("div");
  list.className = "mcp-sublist";
  const listPrefix = label.toLowerCase();

  const emptyState = renderEmptySublist("No entries.");

  const syncEmptyState = () => {
    const hasRows = list.querySelector(".mcp-key-value-row");
    if (hasRows) {
      emptyState.remove();
      return;
    }

    if (!list.contains(emptyState)) {
      list.append(emptyState);
    }
  };

  const renumberRows = () => {
    Array.from(list.querySelectorAll(".mcp-key-value-row")).forEach((row, index) => {
      row.dataset.index = String(index);
      const keyInput = row.querySelector('input[data-role="key"]');
      const valueInput = row.querySelector('input[data-role="value"]');
      if (keyInput) {
        keyInput.dataset.fieldKey = `${listPrefix}-key-${index}`;
      }
      if (valueInput) {
        valueInput.dataset.fieldKey = `${listPrefix}-value-${index}`;
      }
    });
  };

  const createRow = (entry = { key: "", value: "" }) => {
    const row = document.createElement("div");
    row.className = "env-row mcp-key-value-row";

    const keyInput = document.createElement("input");
    keyInput.className = "field-input";
    keyInput.type = "text";
    keyInput.placeholder = "Key";
    keyInput.value = entry.key;
    keyInput.dataset.role = "key";
    disableTextAssistance(keyInput);

    const valueInput = document.createElement("input");
    valueInput.className = "field-input";
    valueInput.type = "text";
    valueInput.placeholder = "Value";
    valueInput.value = entry.value;
    valueInput.dataset.role = "value";
    disableTextAssistance(valueInput);

    const syncEntry = () => {
      const index = Number(row.dataset.index);
      const next = (getEntries() || []).map((item, itemIndex) =>
        itemIndex === index
          ? { key: keyInput.value, value: valueInput.value }
          : item,
      );
      onChange(next);
      deps.markStateChanged();
    };

    const insertAfterRow = () => {
      const index = Number(row.dataset.index);
      const next = [...(getEntries() || [])];
      next.splice(index + 1, 0, { key: "", value: "" });
      onChange(next);

      const newRow = createRow();
      row.after(newRow);
      renumberRows();
      syncEmptyState();
      deps.markStateChanged();
      focusNearestMcpListField(`${listPrefix}-key-${index + 1}`);
    };

    keyInput.addEventListener("input", syncEntry);
    valueInput.addEventListener("input", syncEntry);
    keyInput.addEventListener("keydown", (event) => {
      if (event.key !== "Enter") {
        return;
      }

      event.preventDefault();
      insertAfterRow();
    });
    valueInput.addEventListener("keydown", (event) => {
      if (event.key !== "Enter") {
        return;
      }

      event.preventDefault();
      insertAfterRow();
    });

    const removeButton = document.createElement("button");
    removeButton.type = "button";
    removeButton.className = "secondary-button compact-button danger-button";
    removeButton.textContent = "Remove";
    removeButton.addEventListener("click", () => {
      const index = Number(row.dataset.index);
      const next = (getEntries() || []).filter((_, itemIndex) => itemIndex !== index);
      onChange(next);

      const focusTarget =
        row.nextElementSibling?.querySelector('input[data-role="key"]') ??
        row.previousElementSibling?.querySelector('input[data-role="key"]');

      row.remove();
      renumberRows();
      syncEmptyState();
      deps.markStateChanged();

      if (focusTarget instanceof HTMLElement) {
        focusTarget.focus({ preventScroll: true });
      }
    });

    row.append(keyInput, valueInput, removeButton);
    return row;
  };

  const header = renderListHeader(label, "Add", () => {
    const next = [...(getEntries() || []), { key: "", value: "" }];
    onChange(next);
    list.append(createRow());
    renumberRows();
    syncEmptyState();
    deps.markStateChanged();
    focusNearestMcpListField(`${listPrefix}-key-${next.length - 1}`);
  });

  const entries = getEntries() || [];
  entries.forEach((entry) => {
    list.append(createRow(entry));
  });
  renumberRows();
  syncEmptyState();

  field.append(header, list);
  return field;
}

function renderListHeader(label, actionLabel, onAction) {
  const wrapper = document.createElement("div");
  wrapper.className = "panel-toolbar mcp-sublist-toolbar";

  const title = document.createElement("span");
  title.className = "field-label";
  title.textContent = label;

  const button = document.createElement("button");
  button.type = "button";
  button.className = "secondary-button compact-button";
  button.textContent = actionLabel;
  button.addEventListener("click", onAction);

  wrapper.append(title, button);
  return wrapper;
}

function renderEmptySublist(text) {
  const empty = document.createElement("div");
  empty.className = "muted-panel mcp-sublist-empty";
  empty.textContent = text;
  return empty;
}

function createEmptyServer() {
  return {
    uiId: nextMcpServerUiId(),
    name: nextServerName(),
    transport: "stdio",
    command: "",
    args: [],
    cwd: "",
    envVars: [],
    url: "",
    headers: [],
    timeout: null,
    rawServer: {},
  };
}

function getSelectedMcpServer() {
  return (
    state.mcpServers.find(
      (server) => server.uiId === state.selectedMcpServerUiId,
    ) ?? null
  );
}

function ensureSelectedMcpServer() {
  if (state.mcpServers.length === 0) {
    state.selectedMcpServerUiId = null;
    return;
  }

  if (!getSelectedMcpServer()) {
    state.selectedMcpServerUiId = state.mcpServers[0].uiId;
  }
}

function nextServerName() {
  const existing = new Set(
    state.mcpServers.map((server) => String(server.name || "").trim()),
  );

  let index = 1;
  while (existing.has(`server${index}`)) {
    index += 1;
  }
  return `server${index}`;
}

function buildServerSubtitle(server) {
  if (server.transport === "stdio") {
    return server.command.trim() || "No command configured";
  }

  return server.url.trim() || "No URL configured";
}

function syncMcpPresentation(server) {
  const row = findMcpServerRowElement(server.uiId);
  if (!row) {
    return;
  }

  const title = row.querySelector(".model-row-title strong");
  const meta = row.querySelector(".model-row-meta");
  const badge = row.querySelector(".warning-chip");

  if (title) {
    title.textContent = server.name.trim() || "Unnamed server";
  }
  if (meta) {
    meta.textContent = buildServerSubtitle(server);
  }
  if (badge) {
    badge.textContent = transportLabel(server.transport);
  }
  row.classList.toggle("mcp-server-row-disabled", !server.enabled);
}

function findMcpServerRowElement(uiId) {
  return (
    Array.from(document.querySelectorAll(".mcp-server-row")).find(
      (row) => row.dataset.uiId === uiId,
    ) ?? null
  );
}

function transportLabel(transport) {
  switch (transport) {
    case "http":
      return "HTTP";
    case "sse":
      return "SSE";
    default:
      return "STDIO";
  }
}

function normalizeTransport(value) {
  return ["stdio", "http", "sse"].includes(value) ? value : "stdio";
}

function normalizeKeyValueEntries(entries) {
  if (!Array.isArray(entries)) {
    return [];
  }

  return entries.map((entry) => ({
    key: String(entry?.key || ""),
    value: String(entry?.value || ""),
  }));
}

function nextMcpServerUiId() {
  return window.crypto?.randomUUID?.() ?? `mcp-${Date.now()}-${Math.random()}`;
}

async function browseForCommandPath(currentValue) {
  return openPathDialog({
    multiple: false,
    directory: false,
    ...(absolutePathOrNull(currentValue)
      ? { defaultPath: absolutePathOrNull(currentValue) }
      : {}),
  });
}

async function browseForDirectoryPath(currentValue) {
  return openPathDialog({
    multiple: false,
    directory: true,
    ...(absolutePathOrNull(currentValue)
      ? { defaultPath: absolutePathOrNull(currentValue) }
      : {}),
  });
}

function absolutePathOrNull(value) {
  const trimmed = String(value || "").trim();
  if (!trimmed) {
    return null;
  }

  if (trimmed.startsWith("/")) {
    return trimmed;
  }

  if (/^[A-Za-z]:[\\/]/.test(trimmed)) {
    return trimmed;
  }

  if (trimmed.startsWith("\\\\")) {
    return trimmed;
  }

  return null;
}

function focusMcpInspector() {
  if (state.mcpInspectorFlashTimer) {
    window.clearTimeout(state.mcpInspectorFlashTimer);
    state.mcpInspectorFlashTimer = null;
  }

  elements.mcpInspectorContent.scrollIntoView({
    block: "start",
    behavior: "smooth",
  });
  elements.mcpInspectorContent.classList.add("mcp-inspector-focus");

  state.mcpInspectorFlashTimer = window.setTimeout(() => {
    elements.mcpInspectorContent.classList.remove("mcp-inspector-focus");
    state.mcpInspectorFlashTimer = null;
  }, 900);

  window.requestAnimationFrame(() => {
    const target = elements.mcpInspectorContent.querySelector(
      '[data-mcp-primary-field="true"], input, select, textarea, button',
    );
    if (target instanceof HTMLInputElement) {
      target.focus({ preventScroll: true });
      target.select();
      return;
    }
    if (target instanceof HTMLElement) {
      target.focus({ preventScroll: true });
    }
  });
}

function focusMcpField(fieldKey) {
  if (!fieldKey) {
    return;
  }

  window.requestAnimationFrame(() => {
    const target = elements.mcpInspectorContent.querySelector(
      `[data-field-key="${fieldKey}"]`,
    );
    if (target instanceof HTMLInputElement) {
      target.focus({ preventScroll: true });
      target.select();
      return;
    }
    if (target instanceof HTMLElement) {
      target.focus({ preventScroll: true });
    }
  });
}

function focusNearestMcpListField(fieldKey) {
  if (!fieldKey) {
    return;
  }

  window.requestAnimationFrame(() => {
    focusMcpField(fieldKey);
  });
}

function captureMcpInspectorFocus(server) {
  if (!server) {
    return null;
  }

  const active = document.activeElement;
  if (!(active instanceof HTMLElement)) {
    return null;
  }

  if (!elements.mcpInspectorContent.contains(active)) {
    return null;
  }

  const fieldKey = active.dataset?.fieldKey;
  if (!fieldKey) {
    return null;
  }

  const focus = {
    serverUiId: server.uiId,
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

function restoreMcpInspectorFocus(preservedFocus) {
  if (!preservedFocus) {
    return;
  }

  const selectedServer = state.mcpServers.find(
    (server) => server.uiId === preservedFocus.serverUiId,
  );
  if (
    !selectedServer ||
    state.selectedMcpServerUiId !== preservedFocus.serverUiId
  ) {
    return;
  }

  const selector = `[data-field-key="${preservedFocus.fieldKey}"]`;
  const field = elements.mcpInspectorContent.querySelector(selector);
  if (!(field instanceof HTMLElement)) {
    return;
  }

  field.focus({ preventScroll: true });

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
