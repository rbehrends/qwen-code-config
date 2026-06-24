import { state } from "./shared.js";

export function disableTextAssistance(input) {
  input.autocomplete = "off";
  input.autocapitalize = "off";
  input.spellcheck = false;
  input.setAttribute("autocorrect", "off");
}

// This is a display-only fallback for the editor. Rust remains authoritative
// for canonical model naming in saved snapshots, previews, and model drafts.
export function prettifyModelName(modelId) {
  // These backend-sourced rewrite rules run after generic separator handling and
  // first-letter capitalization, so expressions like ^gpt oss\b target the
  // prettified display string instead of the raw model id.
  const words = String(modelId || "")
    .split(/[\/_:-]+/)
    .filter(Boolean)
    .map((segment) => {
      if (/^\d+$/.test(segment)) {
        return segment;
      }
      return segment.charAt(0).toUpperCase() + segment.slice(1);
    });

  return applyModelNameRewriteRules(words.join(" "));
}

function applyModelNameRewriteRules(value) {
  return state.modelNameRewriteRules.reduce((current, rule) => {
    const regex = new RegExp(rule.pattern, "g");

    if (typeof rule.replacement === "string") {
      return current.replace(regex, rule.replacement);
    }

    const uppercaseCaptures = Array.isArray(rule.uppercaseCaptures)
      ? rule.uppercaseCaptures
      : [];
    return current.replace(regex, (...args) => {
      const captures = args.slice(1, -2);
      return captures
        .map((capture, index) =>
          uppercaseCaptures.includes(index + 1)
            ? String(capture).toUpperCase()
            : String(capture),
        )
        .join("");
    });
  }, value);
}

export function normalizeModelNameRewriteRules(rules) {
  if (!Array.isArray(rules)) {
    return [];
  }

  return rules
    .map((rule) => ({
      pattern: String(rule?.pattern || ""),
      replacement:
        typeof rule?.replacement === "string" ? String(rule.replacement) : null,
      uppercaseCaptures: Array.isArray(rule?.uppercaseCaptures)
        ? rule.uppercaseCaptures
            .map((value) => Number.parseInt(value, 10))
            .filter((value) => Number.isInteger(value) && value > 0)
        : [],
    }))
    .filter((rule) => rule.pattern);
}

export function filterCatalogModels(models, query, mode) {
  const normalizedQuery = String(query || "")
    .trim()
    .toLowerCase();
  if (!normalizedQuery) {
    return models;
  }

  return models.filter((model) => {
    const candidate = String(model.id || "").toLowerCase();
    return mode === "fuzzy"
      ? fuzzyMatches(candidate, normalizedQuery)
      : candidate.includes(normalizedQuery);
  });
}

export function fuzzyMatches(candidate, query) {
  let queryIndex = 0;

  for (
    let candidateIndex = 0;
    candidateIndex < candidate.length;
    candidateIndex += 1
  ) {
    if (candidate[candidateIndex] === query[queryIndex]) {
      queryIndex += 1;
      if (queryIndex === query.length) {
        return true;
      }
    }
  }

  return query.length === 0;
}

export function preservedKeysCount(object, knownKeys) {
  return Object.keys(object || {}).filter((key) => !knownKeys.includes(key))
    .length;
}

export function applyLayoutDensity(value) {
  document.body.dataset.density = value;
}

export function applyThemeMode(value) {
  document.body.dataset.theme = value;
}

// These guards only protect local UI interactions such as menu events and
// select inputs. Persisted UI state is normalized authoritatively in Rust.
export function normalizeLayoutDensity(value) {
  return ["compact", "comfortable", "spacious"].includes(value)
    ? value
    : "compact";
}

export function normalizeThemeMode(value) {
  return ["light", "dark", "system"].includes(value) ? value : "system";
}

export function parseSnapshotJson(jsonText) {
  try {
    return JSON.parse(jsonText);
  } catch {
    return {};
  }
}

export function cloneJsonValue(value) {
  return JSON.parse(JSON.stringify(value ?? {}));
}
