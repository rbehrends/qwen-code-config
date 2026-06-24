use crate::types::{
    CatalogModel, ModelEntry, ModelNameRewriteRule, ProviderDraftProfile, ReasoningMode,
    SupportedProtocol,
};
use regex::{Captures, Regex};
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    sync::LazyLock,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static MODEL_DRAFT_COUNTER: AtomicU64 = AtomicU64::new(1);
static MODEL_NAME_REWRITE_RULES: LazyLock<Vec<ModelNameRewritePattern>> = LazyLock::new(|| {
    vec![
        ModelNameRewritePattern::replace(r"^Gpt Oss\b", "GPT OSS"),
        ModelNameRewritePattern::replace(r"^Gpt\b", "GPT"),
        ModelNameRewritePattern::replace(r"^Glm\b", "GLM"),
        ModelNameRewritePattern::replace(r"^Mimo\b", "MiMo"),
        ModelNameRewritePattern::replace(r"^Minimax\b", "MiniMax"),
        ModelNameRewritePattern::replace(r"^Deepseek\b", "DeepSeek"),
        ModelNameRewritePattern::replace(r"\b([0-9]+)b\b", "$1B"),
        ModelNameRewritePattern::uppercase_captures(r"\b([A-Za-z])([0-9]+)([A-Za-z])\b", &[1, 3]),
    ]
});
const COMMON_MODEL_NAME_CAPITALIZATIONS: [(&str, &str); 4] = [
    ("glm", "GLM"),
    ("mimo", "MiMo"),
    ("minimax", "MiniMax"),
    ("deepseek", "DeepSeek"),
];

pub(crate) fn build_model_draft(
    profile: ProviderDraftProfile,
    source: CatalogModel,
    has_existing_default: bool,
) -> Result<ModelEntry, String> {
    let id = source.id.trim().to_string();
    if id.is_empty() {
        return Err("Model ids cannot be empty.".to_string());
    }

    Ok(ModelEntry {
        ui_id: next_model_draft_id(),
        protocol: profile.protocol,
        id: id.clone(),
        name: normalized_model_name(source.name.trim(), &id),
        base_url: profile.base_url.trim().to_string(),
        env_key: profile.env_key.trim().to_string(),
        context_window_size: source.context_window_size,
        temperature: None,
        top_p: None,
        max_tokens: None,
        reasoning_mode: ReasoningMode::Default,
        reasoning_effort: None,
        reasoning_budget_tokens: None,
        sampling_params: Map::new(),
        extra_body: Map::new(),
        raw_model: Value::Object(Map::new()),
        is_default: !has_existing_default,
        is_duplicate: false,
    })
}

pub(crate) fn mark_duplicate_models(models: &mut [ModelEntry]) {
    let counts = duplicate_counts(models);

    for model in models.iter_mut() {
        let key = duplicate_key(&model.protocol, &model.id, &model.base_url);
        model.is_duplicate = counts.get(&key).copied().unwrap_or_default() > 1;
    }
}

pub(crate) fn normalized_model_name(name: &str, id: &str) -> String {
    if name.is_empty() {
        prettify_model_name(id)
    } else {
        name.to_string()
    }
}

pub(crate) fn prettify_model_name(id: &str) -> String {
    let words = id
        .split(['/', '-', '_', ':'])
        .filter(|segment| !segment.is_empty())
        .map(prettify_segment)
        .collect::<Vec<_>>();
    apply_model_name_rewrite_rules(&words.join(" "))
}

pub(crate) fn duplicate_key(protocol: &SupportedProtocol, id: &str, base_url: &str) -> String {
    format!("{}\0{}\0{}", protocol.as_str(), id.trim(), base_url.trim())
}

pub(crate) fn common_model_name_capitalizations() -> BTreeMap<String, String> {
    COMMON_MODEL_NAME_CAPITALIZATIONS
        .iter()
        .map(|(source, replacement)| (source.to_string(), replacement.to_string()))
        .collect()
}

pub(crate) fn model_name_rewrite_rules() -> Vec<ModelNameRewriteRule> {
    MODEL_NAME_REWRITE_RULES
        .iter()
        .map(ModelNameRewritePattern::to_public_rule)
        .collect()
}

pub(crate) fn apply_effective_default_flags(models: &mut [ModelEntry], json: &Value) {
    let selected_protocol = json
        .get("security")
        .and_then(|value| value.get("auth"))
        .and_then(|value| value.get("selectedType"))
        .and_then(Value::as_str)
        .and_then(SupportedProtocol::parse);
    let selected_model_id = json
        .get("model")
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str);

    let mut matched_default = false;

    for model in models {
        let is_match = !matched_default
            && selected_protocol.as_ref() == Some(&model.protocol)
            && selected_model_id == Some(model.id.trim());
        model.is_default = is_match;
        if is_match {
            matched_default = true;
        }
    }
}

fn duplicate_counts(models: &[ModelEntry]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();

    for model in models {
        let key = duplicate_key(&model.protocol, &model.id, &model.base_url);
        *counts.entry(key).or_insert(0usize) += 1;
    }

    counts
}

fn next_model_draft_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = MODEL_DRAFT_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("draft-{nanos:x}-{counter:x}")
}

fn prettify_segment(segment: &str) -> String {
    let lower = segment.to_ascii_lowercase();
    if lower.chars().all(|character| character.is_ascii_digit()) {
        return segment.to_string();
    }

    let mut characters = segment.chars();
    match characters.next() {
        Some(first) => {
            let mut value = first.to_ascii_uppercase().to_string();
            value.push_str(characters.as_str());
            value
        }
        None => String::new(),
    }
}

fn apply_model_name_rewrite_rules(value: &str) -> String {
    MODEL_NAME_REWRITE_RULES
        .iter()
        .fold(value.to_string(), |current, rule| rule.apply(&current))
}

struct ModelNameRewritePattern {
    regex: Regex,
    pattern: &'static str,
    replacement: Option<&'static str>,
    rust_replacement: Option<String>,
    uppercase_captures: &'static [usize],
}

impl ModelNameRewritePattern {
    fn replace(pattern: &'static str, replacement: &'static str) -> Self {
        Self {
            regex: Regex::new(pattern).expect("valid model-name rewrite regex"),
            pattern,
            replacement: Some(replacement),
            rust_replacement: Some(normalize_rust_replacement(replacement)),
            uppercase_captures: &[],
        }
    }

    fn uppercase_captures(pattern: &'static str, uppercase_captures: &'static [usize]) -> Self {
        Self {
            regex: Regex::new(pattern).expect("valid model-name rewrite regex"),
            pattern,
            replacement: None,
            rust_replacement: None,
            uppercase_captures,
        }
    }

    fn apply(&self, value: &str) -> String {
        if let Some(replacement) = self.rust_replacement.as_deref() {
            return self.regex.replace_all(value, replacement).into_owned();
        }

        self.regex
            .replace_all(value, |captures: &Captures<'_>| {
                uppercase_capture_match(captures, self.uppercase_captures)
            })
            .into_owned()
    }

    fn to_public_rule(&self) -> ModelNameRewriteRule {
        ModelNameRewriteRule {
            pattern: self.pattern.to_string(),
            replacement: self.replacement.map(str::to_string),
            uppercase_captures: self.uppercase_captures.to_vec(),
        }
    }
}

fn uppercase_capture_match(captures: &Captures<'_>, uppercase_captures: &[usize]) -> String {
    let mut value = String::new();

    for index in 1..captures.len() {
        let Some(segment) = captures.get(index).map(|capture| capture.as_str()) else {
            continue;
        };

        if uppercase_captures.contains(&index) {
            value.push_str(&segment.to_ascii_uppercase());
        } else {
            value.push_str(segment);
        }
    }

    value
}

fn normalize_rust_replacement(replacement: &str) -> String {
    let mut normalized = String::new();
    let mut characters = replacement.chars().peekable();

    while let Some(character) = characters.next() {
        if character == '$' {
            let mut digits = String::new();
            while let Some(next) = characters.peek() {
                if next.is_ascii_digit() {
                    digits.push(*next);
                    characters.next();
                } else {
                    break;
                }
            }

            if !digits.is_empty() {
                normalized.push_str("${");
                normalized.push_str(&digits);
                normalized.push('}');
                continue;
            }
        }

        normalized.push(character);
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_model_draft_normalizes_provider_and_name() {
        let draft = build_model_draft(
            ProviderDraftProfile {
                base_url: " https://example.com/v1 ".to_string(),
                env_key: " EXAMPLE_KEY ".to_string(),
                protocol: SupportedProtocol::Openai,
            },
            CatalogModel {
                id: " test-model ".to_string(),
                name: String::new(),
                context_window_size: Some(1024),
                supports_vision: false,
            },
            false,
        )
        .unwrap();

        assert_eq!(draft.id, "test-model");
        assert_eq!(draft.name, "Test Model");
        assert_eq!(draft.base_url, "https://example.com/v1");
        assert_eq!(draft.env_key, "EXAMPLE_KEY");
        assert_eq!(draft.context_window_size, Some(1024));
        assert!(draft.is_default);
        assert!(draft.ui_id.starts_with("draft-"));
    }

    #[test]
    fn build_model_draft_rejects_empty_ids() {
        let error = build_model_draft(
            ProviderDraftProfile {
                base_url: String::new(),
                env_key: String::new(),
                protocol: SupportedProtocol::Anthropic,
            },
            CatalogModel {
                id: "   ".to_string(),
                name: String::new(),
                context_window_size: None,
                supports_vision: false,
            },
            true,
        )
        .unwrap_err();

        assert!(error.contains("cannot be empty"));
    }

    #[test]
    fn prettify_model_name_preserves_periods_inside_segments() {
        assert_eq!(prettify_model_name("kimi-k2.6"), "Kimi K2.6");
        assert_eq!(
            prettify_model_name("qwen/qwen3.5-coder"),
            "Qwen Qwen3.5 Coder"
        );
    }

    #[test]
    fn prettify_model_name_applies_brand_capitalization_to_first_word() {
        assert_eq!(prettify_model_name("glm-4.5"), "GLM 4.5");
        assert_eq!(prettify_model_name("mimo-7b"), "MiMo 7B");
        assert_eq!(prettify_model_name("minimax-m1"), "MiniMax M1");
        assert_eq!(prettify_model_name("deepseek-r1"), "DeepSeek R1");
        assert_eq!(prettify_model_name("qwen/deepseek-r1"), "Qwen Deepseek R1");
    }

    #[test]
    fn prettify_model_name_applies_ordered_regex_rewrites() {
        assert_eq!(prettify_model_name("gpt-5.4"), "GPT 5.4");
        assert_eq!(prettify_model_name("gpt-oss-20b"), "GPT OSS 20B");
        assert_eq!(prettify_model_name("mixtral-a3b"), "Mixtral A3B");
    }

    #[test]
    fn common_model_name_capitalizations_exports_supported_names() {
        let names = common_model_name_capitalizations();
        assert_eq!(names.get("glm").map(String::as_str), Some("GLM"));
        assert_eq!(names.get("mimo").map(String::as_str), Some("MiMo"));
        assert_eq!(names.get("minimax").map(String::as_str), Some("MiniMax"));
        assert_eq!(names.get("deepseek").map(String::as_str), Some("DeepSeek"));
    }

    #[test]
    fn model_name_rewrite_rules_exports_ordered_rules() {
        let rules = model_name_rewrite_rules();

        assert_eq!(rules[0].pattern, "^Gpt Oss\\b");
        assert_eq!(rules[0].replacement.as_deref(), Some("GPT OSS"));
        assert_eq!(rules[6].pattern, "\\b([0-9]+)b\\b");
        assert_eq!(rules[6].replacement.as_deref(), Some("$1B"));
        assert_eq!(rules[7].uppercase_captures, vec![1, 3]);
    }

    #[test]
    fn normalize_rust_replacement_handles_suffix_text() {
        assert_eq!(normalize_rust_replacement("$1B"), "${1}B");
        assert_eq!(normalize_rust_replacement("$1$2"), "${1}${2}");
    }
}
