#![allow(dead_code)]

use crate::autocomplete::path_parser::{PathSegment, parse_path};
use crate::query::ResultType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSource {
    /// Last successful jq output cache.
    ResultCache,
    /// Original input JSON from file.
    OriginalJson,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetLevel {
    /// Suggest fields from the exact value at the resolved path.
    ValueAtPath {
        source: TargetSource,
        segments: Vec<PathSegment>,
    },
    /// Suggest fields from array elements at a path (used for `...[]` level suggestions).
    ArrayElementsAtPath {
        source: TargetSource,
        array_segments: Vec<PathSegment>,
    },
    /// Deterministic fallback when pipe input is unknown in non-executing contexts.
    AllKnownFields,
    /// No viable source is available.
    None,
}

/// Inputs needed to resolve "what JSON level should autocomplete inspect?"
#[derive(Debug, Clone)]
pub struct ResolveTargetInput<'a> {
    /// Parsed expression slice near cursor (e.g. `.services[].tasks.`).
    pub path_context: &'a str,
    /// True when boundary before the path is a pipe (`|`).
    pub is_after_pipe: bool,
    /// True when cursor is in contexts where cache may be stale (map/select/builders).
    pub is_non_executing: bool,
    /// True when cursor is at logical end (or trailing whitespace only).
    pub is_at_end: bool,
    /// True inside element-context function bodies (map/select/...).
    pub is_in_element_context: bool,
    /// Type of current cached result (used for streaming vs non-streaming handling).
    pub result_type: Option<&'a ResultType>,
    /// Whether last_successful_result cache exists.
    pub has_result_cache: bool,
    /// Whether original input JSON exists.
    pub has_original_json: bool,
    /// Optional provenance path for the most recent array iterator.
    pub array_provenance: Option<&'a [PathSegment]>,
}

/// Resolve target level/source for autocomplete.
///
/// This isolates context routing from suggestion generation so key enrichment
/// policy can be applied consistently across query shapes.
pub fn resolve_target_level(input: &ResolveTargetInput<'_>) -> TargetLevel {
    let mut parsed_path = parse_path(input.path_context);

    // Keep existing Phase 7 behavior.
    let is_streaming = matches!(input.result_type, Some(ResultType::DestructuredObjects));
    if input.is_in_element_context && !is_streaming {
        parsed_path.segments.insert(0, PathSegment::ArrayIterator);
    }

    if parsed_path.segments.is_empty() && input.is_after_pipe {
        return TargetLevel::AllKnownFields;
    }

    let Some(source) = choose_source(input) else {
        return TargetLevel::None;
    };

    if matches!(parsed_path.segments.last(), Some(PathSegment::ArrayIterator)) {
        parsed_path.segments.pop();
        return TargetLevel::ArrayElementsAtPath {
            source,
            array_segments: parsed_path.segments,
        };
    }

    if input.is_in_element_context
        && parsed_path.segments.is_empty()
        && let Some(provenance) = input.array_provenance
        && !provenance.is_empty()
    {
        return TargetLevel::ArrayElementsAtPath {
            source,
            array_segments: provenance.to_vec(),
        };
    }

    TargetLevel::ValueAtPath {
        source,
        segments: parsed_path.segments,
    }
}

fn choose_source(input: &ResolveTargetInput<'_>) -> Option<TargetSource> {
    // Mid-query edits should use original JSON because the cache reflects a different cursor position.
    if !input.is_at_end {
        return input.has_original_json.then_some(TargetSource::OriginalJson);
    }

    // Non-executing contexts prefer cache when available, then original JSON.
    if input.is_non_executing
        && let Some(source) = prefer_cache_then_original(
            input.has_result_cache,
            input.has_original_json,
        )
    {
        return Some(source);
    }

    // Executing contexts at end prefer cache (current query result).
    if input.has_result_cache {
        return Some(TargetSource::ResultCache);
    }

    input.has_original_json.then_some(TargetSource::OriginalJson)
}

fn prefer_cache_then_original(
    has_result_cache: bool,
    has_original_json: bool,
) -> Option<TargetSource> {
    if has_result_cache {
        Some(TargetSource::ResultCache)
    } else if has_original_json {
        Some(TargetSource::OriginalJson)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "target_level_tests.rs"]
mod target_level_tests;
