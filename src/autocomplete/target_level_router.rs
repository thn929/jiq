use super::autocomplete_state::Suggestion;
use super::json_navigator::navigate;
use super::result_analyzer::ResultAnalyzer;
use super::target_level::{
    ResolveTargetInput, TargetLevel, TargetSource, resolve_target_level,
};
use crate::query::ResultType;
use serde_json::Value;

/// Resolve target level and generate nested field suggestions from the chosen source.
///
/// Returns `None` when target/source/path can't be resolved; callers keep existing fallback behavior.
#[allow(clippy::too_many_arguments)]
pub fn get_nested_target_suggestions(
    path_context: &str,
    needs_leading_dot: bool,
    suppress_array_brackets: bool,
    is_in_element_context: bool,
    is_after_pipe: bool,
    is_non_executing: bool,
    is_at_end: bool,
    result_type: Option<&ResultType>,
    result_parsed: Option<&Value>,
    original_json: Option<&Value>,
) -> Option<Vec<Suggestion>> {
    let target = resolve_target_level(&ResolveTargetInput {
        path_context,
        is_after_pipe,
        is_non_executing,
        is_at_end,
        is_in_element_context,
        result_type,
        has_result_cache: result_parsed.is_some(),
        has_original_json: original_json.is_some(),
    });

    match target {
        TargetLevel::AllKnownFields | TargetLevel::None => None,
        TargetLevel::ValueAtPath { source, segments } => {
            let json = select_source_json(source, result_parsed, original_json)?;
            let navigated = navigate(json, &segments)?;
            Some(ResultAnalyzer::analyze_value(
                navigated,
                needs_leading_dot,
                suppress_array_brackets,
            ))
        }
        TargetLevel::ArrayElementsAtPath {
            source,
            array_segments,
        } => {
            let json = select_source_json(source, result_parsed, original_json)?;
            let navigated = navigate(json, &array_segments)?;
            match navigated {
                Value::Array(_) => Some(ResultAnalyzer::analyze_value(
                    navigated,
                    needs_leading_dot,
                    true,
                )),
                _ => None,
            }
        }
    }
}

fn select_source_json<'a>(
    source: TargetSource,
    result_parsed: Option<&'a Value>,
    original_json: Option<&'a Value>,
) -> Option<&'a Value> {
    match source {
        TargetSource::ResultCache => result_parsed,
        TargetSource::OriginalJson => original_json,
    }
}
