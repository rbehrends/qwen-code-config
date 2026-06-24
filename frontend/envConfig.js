import { elements, state } from "./shared.js";

let deps = null;

export function initializeEnvConfig(nextDeps) {
  deps = nextDeps;
}

export function renderEnvVars() {
  elements.envList.replaceChildren();

  if (state.envVars.length === 0) {
    const empty = document.createElement("div");
    empty.className = "env-empty";
    empty.textContent = "No fallback environment variables configured.";
    elements.envList.append(empty);
    return;
  }

  state.envVars.forEach((envVar, index) => {
    const row = document.createElement("div");
    row.className = "env-row";

    const keyInput = document.createElement("input");
    keyInput.type = "text";
    keyInput.placeholder = "API_KEY";
    keyInput.autocomplete = "off";
    keyInput.autocapitalize = "off";
    keyInput.spellcheck = false;
    keyInput.setAttribute("autocorrect", "off");
    keyInput.value = envVar.key;
    keyInput.addEventListener("input", () => {
      state.envVars[index].key = keyInput.value;
      deps.markStateChanged();
    });

    const valueInput = document.createElement("input");
    valueInput.type = envVar.revealed ? "text" : "password";
    valueInput.placeholder = "Value";
    valueInput.autocomplete = "off";
    valueInput.autocapitalize = "off";
    valueInput.spellcheck = false;
    valueInput.setAttribute("autocorrect", "off");
    valueInput.value = envVar.value;
    valueInput.addEventListener("input", () => {
      state.envVars[index].value = valueInput.value;
      deps.markStateChanged();
    });

    const actionGroup = document.createElement("div");
    actionGroup.className = "env-actions";

    const revealButton = document.createElement("button");
    revealButton.type = "button";
    revealButton.className = "secondary-button compact-button";
    revealButton.textContent = envVar.revealed ? "Hide" : "Reveal";
    revealButton.addEventListener("click", () => {
      state.envVars[index].revealed = !state.envVars[index].revealed;
      renderEnvVars();
    });

    const removeButton = document.createElement("button");
    removeButton.type = "button";
    removeButton.className = "secondary-button compact-button danger-button";
    removeButton.textContent = "Remove";
    removeButton.addEventListener("click", () => {
      state.envVars.splice(index, 1);
      renderEnvVars();
      deps.markStateChanged();
    });

    actionGroup.append(revealButton, removeButton);
    row.append(keyInput, valueInput, actionGroup);
    elements.envList.append(row);
  });
}

export function serializeEnvVars() {
  return state.envVars.map(({ key, value }) => ({
    key,
    value,
  }));
}

export function nextEnvKey() {
  const base = "NEW_API_KEY";
  const existing = new Set(state.envVars.map((envVar) => envVar.key));

  if (!existing.has(base)) {
    return base;
  }

  for (let index = 2; ; index += 1) {
    const candidate = `${base}_${index}`;

    if (!existing.has(candidate)) {
      return candidate;
    }
  }
}
