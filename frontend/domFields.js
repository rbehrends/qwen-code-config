import { disableTextAssistance } from "./utils.js";

export function createField(labelText, value, onInput, options = {}) {
  const label = document.createElement("label");

  const title = document.createElement("span");
  title.className = "field-label";
  title.textContent = labelText;

  const input = document.createElement("input");
  input.type = "text";
  input.className = "field-input";
  if (options.fieldKey) {
    input.dataset.fieldKey = options.fieldKey;
  }
  input.value = value ?? "";
  input.placeholder = options.placeholder ?? "";
  disableTextAssistance(input);
  input.addEventListener("input", () => onInput(input.value));

  label.append(title, input);
  return label;
}

export function createSelectField(labelText, value, choices, onChange, options = {}) {
  const label = document.createElement("label");

  const title = document.createElement("span");
  title.className = "field-label";
  title.textContent = labelText;

  const select = document.createElement("select");
  select.className = "field-input";
  if (options.fieldKey) {
    select.dataset.fieldKey = options.fieldKey;
  }

  choices.forEach((choice) => {
    const option = document.createElement("option");
    option.value = choice.value;
    option.textContent = choice.label;
    option.selected = choice.value === value;
    select.append(option);
  });

  select.addEventListener("change", () => onChange(select.value));

  label.append(title, select);
  return label;
}

export function createNumberField(labelText, value, onChange, options = {}) {
  const label = document.createElement("label");

  const title = document.createElement("span");
  title.className = "field-label";
  title.textContent = labelText;

  const input = document.createElement("input");
  input.type = "number";
  input.className = "field-input";
  if (options.fieldKey) {
    input.dataset.fieldKey = options.fieldKey;
  }
  input.value = value ?? "";
  if (options.integer) {
    input.step = "1";
    input.min = "0";
  } else {
    input.step = "0.1";
  }
  input.addEventListener("input", () => {
    if (input.value === "") {
      onChange(null);
      return;
    }

    const numeric = options.integer
      ? Number.parseInt(input.value, 10)
      : Number.parseFloat(input.value);
    onChange(Number.isFinite(numeric) ? numeric : null);
  });

  label.append(title, input);
  return label;
}
