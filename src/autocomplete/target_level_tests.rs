use super::*;

fn base_input(path_context: &str) -> ResolveTargetInput<'_> {
    ResolveTargetInput {
        path_context,
        is_after_pipe: false,
        is_non_executing: false,
        is_at_end: true,
        is_in_element_context: false,
        result_type: Some(&ResultType::Object),
        has_result_cache: true,
        has_original_json: true,
    }
}

#[derive(Debug)]
struct Case<'a> {
    name: &'a str,
    input: ResolveTargetInput<'a>,
    expected: TargetLevel,
}

#[test]
fn decision_table_resolve_target_level() {
    let mut non_executing = base_input(".services.");
    non_executing.is_non_executing = true;

    let mut non_executing_no_cache = base_input(".services.");
    non_executing_no_cache.is_non_executing = true;
    non_executing_no_cache.has_result_cache = false;

    let mut non_executing_no_sources = base_input(".services.");
    non_executing_no_sources.is_non_executing = true;
    non_executing_no_sources.has_result_cache = false;
    non_executing_no_sources.has_original_json = false;

    let mut after_pipe = base_input(".");
    after_pipe.is_after_pipe = true;

    let mut middle_of_query = base_input(".services.");
    middle_of_query.is_at_end = false;

    let mut middle_of_query_no_original = base_input(".services.");
    middle_of_query_no_original.is_at_end = false;
    middle_of_query_no_original.has_original_json = false;

    let mut element_non_streaming = base_input(".deploymentConfiguration.");
    element_non_streaming.is_in_element_context = true;
    element_non_streaming.result_type = Some(&ResultType::ArrayOfObjects);

    let mut element_streaming = base_input(".deploymentConfiguration.");
    element_streaming.is_in_element_context = true;
    element_streaming.result_type = Some(&ResultType::DestructuredObjects);

    let mut element_non_streaming_trailing_iterator = base_input(".tasks[]");
    element_non_streaming_trailing_iterator.is_in_element_context = true;
    element_non_streaming_trailing_iterator.result_type = Some(&ResultType::ArrayOfObjects);

    let mut no_sources = base_input(".services.");
    no_sources.has_result_cache = false;
    no_sources.has_original_json = false;

    let cases = vec![
        Case {
            name: "executing end resolves from cache",
            input: base_input(".services."),
            expected: TargetLevel::ValueAtPath {
                source: TargetSource::ResultCache,
                segments: vec![PathSegment::Field("services".to_string())],
            },
        },
        Case {
            name: "trailing iterator resolves as array elements",
            input: base_input(".services[]"),
            expected: TargetLevel::ArrayElementsAtPath {
                source: TargetSource::ResultCache,
                array_segments: vec![PathSegment::Field("services".to_string())],
            },
        },
        Case {
            name: "after pipe with empty path resolves to all fields fallback",
            input: after_pipe,
            expected: TargetLevel::AllKnownFields,
        },
        Case {
            name: "non-executing prefers cache",
            input: non_executing,
            expected: TargetLevel::ValueAtPath {
                source: TargetSource::ResultCache,
                segments: vec![PathSegment::Field("services".to_string())],
            },
        },
        Case {
            name: "non-executing falls back to original when cache missing",
            input: non_executing_no_cache,
            expected: TargetLevel::ValueAtPath {
                source: TargetSource::OriginalJson,
                segments: vec![PathSegment::Field("services".to_string())],
            },
        },
        Case {
            name: "non-executing with no sources resolves none",
            input: non_executing_no_sources,
            expected: TargetLevel::None,
        },
        Case {
            name: "middle-of-query uses original source",
            input: middle_of_query,
            expected: TargetLevel::ValueAtPath {
                source: TargetSource::OriginalJson,
                segments: vec![PathSegment::Field("services".to_string())],
            },
        },
        Case {
            name: "middle-of-query with no original resolves none",
            input: middle_of_query_no_original,
            expected: TargetLevel::None,
        },
        Case {
            name: "element context non-streaming prepends iterator",
            input: element_non_streaming,
            expected: TargetLevel::ValueAtPath {
                source: TargetSource::ResultCache,
                segments: vec![
                    PathSegment::ArrayIterator,
                    PathSegment::Field("deploymentConfiguration".to_string()),
                ],
            },
        },
        Case {
            name: "element context streaming does not prepend iterator",
            input: element_streaming,
            expected: TargetLevel::ValueAtPath {
                source: TargetSource::ResultCache,
                segments: vec![PathSegment::Field("deploymentConfiguration".to_string())],
            },
        },
        Case {
            name: "element context with trailing iterator resolves parent array path",
            input: element_non_streaming_trailing_iterator,
            expected: TargetLevel::ArrayElementsAtPath {
                source: TargetSource::ResultCache,
                array_segments: vec![
                    PathSegment::ArrayIterator,
                    PathSegment::Field("tasks".to_string()),
                ],
            },
        },
        Case {
            name: "executing context with no sources resolves none",
            input: no_sources,
            expected: TargetLevel::None,
        },
        Case {
            name: "quoted key path remains value-at-path",
            input: base_input(".services[\"extra-service\"]."),
            expected: TargetLevel::ValueAtPath {
                source: TargetSource::ResultCache,
                segments: vec![
                    PathSegment::Field("services".to_string()),
                    PathSegment::Field("extra-service".to_string()),
                ],
            },
        },
    ];

    for case in cases {
        let actual = resolve_target_level(&case.input);
        assert_eq!(
            actual, case.expected,
            "decision-table case failed: {}",
            case.name
        );
    }
}
