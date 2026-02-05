use super::extract_array_provenance;
use crate::autocomplete::path_parser::PathSegment;

#[test]
fn test_extract_provenance_simple_iterator() {
    let input = ".services[] | select(.status == \"ACTIVE\")";
    let provenance = extract_array_provenance(input).unwrap();
    assert_eq!(provenance, vec![PathSegment::Field("services".to_string())]);
}

#[test]
fn test_extract_provenance_nested_iterator() {
    let input = ".services[].deployments[] | select(.status)";
    let provenance = extract_array_provenance(input).unwrap();
    assert_eq!(
        provenance,
        vec![
            PathSegment::Field("services".to_string()),
            PathSegment::ArrayIterator,
            PathSegment::Field("deployments".to_string()),
        ]
    );
}

#[test]
fn test_extract_provenance_none_when_no_iterator() {
    let input = ".services | select(.status)";
    let provenance = extract_array_provenance(input);
    assert!(provenance.is_none());
}
