use crate::autocomplete::autocomplete_state::JsonFieldType;
use serde_json::Value;
use std::collections::HashSet;

/// Resolved strategy for array key discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayKeyEnrichmentMode {
    /// Existing behavior: infer keys from first object element.
    FirstObject,
    /// Collect unique keys from the first N object elements.
    ScanAhead(usize),
}

const ARRAY_KEY_SCAN_AHEAD_ENV: &str = "JIQ_AUTOCOMPLETE_ARRAY_SCAN_AHEAD";

/// Resolve array-key enrichment mode from environment.
///
/// If `JIQ_AUTOCOMPLETE_ARRAY_SCAN_AHEAD` is valid (> 0), it takes precedence.
pub fn mode_from_env() -> ArrayKeyEnrichmentMode {
    if let Some(scan_size) = scan_ahead_size_from_env() {
        return ArrayKeyEnrichmentMode::ScanAhead(scan_size);
    }

    ArrayKeyEnrichmentMode::FirstObject
}

fn scan_ahead_size_from_env() -> Option<usize> {
    let raw = std::env::var(ARRAY_KEY_SCAN_AHEAD_ENV).ok()?;
    let parsed = raw.parse::<usize>().ok()?;
    (parsed > 0).then_some(parsed)
}

/// Return fields used for array suggestions as `(key, inferred_type)`.
///
/// For `ScanAhead(N)`, this returns unique keys encountered in first N object elements,
/// preserving first-seen key order.
pub fn select_array_fields_for_suggestions(array: &[Value]) -> Vec<(String, JsonFieldType)> {
    match mode_from_env() {
        ArrayKeyEnrichmentMode::FirstObject => select_first_object_fields(array),
        ArrayKeyEnrichmentMode::ScanAhead(scan_size) => select_unique_fields_in_prefix(array, scan_size),
    }
}

fn select_first_object_fields(array: &[Value]) -> Vec<(String, JsonFieldType)> {
    let Some(Value::Object(map)) = array.first() else {
        return Vec::new();
    };

    map.iter()
        .map(|(key, value)| (key.clone(), detect_json_type(value)))
        .collect()
}

fn select_unique_fields_in_prefix(
    array: &[Value],
    scan_size: usize,
) -> Vec<(String, JsonFieldType)> {
    let mut seen = HashSet::new();
    let mut fields = Vec::new();

    for item in array.iter().take(scan_size) {
        let Value::Object(map) = item else {
            continue;
        };

        for (key, value) in map {
            if seen.insert(key.clone()) {
                fields.push((key.clone(), detect_json_type(value)));
            }
        }
    }

    if fields.is_empty() {
        return select_first_object_fields(array);
    }

    fields
}

fn detect_json_type(value: &Value) -> JsonFieldType {
    match value {
        Value::Null => JsonFieldType::Null,
        Value::Bool(_) => JsonFieldType::Boolean,
        Value::Number(_) => JsonFieldType::Number,
        Value::String(_) => JsonFieldType::String,
        Value::Array(arr) => {
            if arr.is_empty() {
                JsonFieldType::Array
            } else {
                let inner_type = detect_json_type(&arr[0]);
                JsonFieldType::ArrayOf(Box::new(inner_type))
            }
        }
        Value::Object(_) => JsonFieldType::Object,
    }
}

#[cfg(test)]
#[path = "array_key_enrichment_strategy_tests.rs"]
mod array_key_enrichment_strategy_tests;
