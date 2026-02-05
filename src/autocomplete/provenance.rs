use super::path_parser::PathSegment;
use super::scan_state::ScanState;

/// Extract the last array-iterator provenance path from text before cursor.
///
/// Returns the path segments leading to the array whose elements are currently
/// in scope (i.e., the segments before the most recent `[]`).
pub fn extract_array_provenance(before_cursor: &str) -> Option<Vec<PathSegment>> {
    let mut state = ScanState::default();
    let mut current_path: Vec<PathSegment> = Vec::new();
    let mut last_array_path: Option<Vec<PathSegment>> = None;

    let mut chars = before_cursor.chars().peekable();
    while let Some(ch) = chars.next() {
        state = state.advance(ch);
        if state.is_in_string() {
            continue;
        }

        match ch {
            '.' => {
                if let Some(&next) = chars.peek() {
                    match next {
                        '[' => {
                            chars.next();
                            if let Some(segment) = parse_bracket_content(&mut chars) {
                                if matches!(segment, PathSegment::ArrayIterator) {
                                    last_array_path = Some(current_path.clone());
                                }
                                current_path.push(segment);
                            }
                        }
                        c if is_field_start_char(c) => {
                            let name = collect_field_name(&mut chars);
                            current_path.push(PathSegment::Field(name));
                        }
                        _ => {}
                    }
                }
            }
            '[' => {
                if let Some(segment) = parse_bracket_content(&mut chars) {
                    if matches!(segment, PathSegment::ArrayIterator) {
                        last_array_path = Some(current_path.clone());
                    }
                    current_path.push(segment);
                }
            }
            '|' | ';' | ',' | ' ' | '\t' | '\n' | '\r' => {
                current_path.clear();
            }
            _ => {}
        }
    }

    last_array_path
}

fn is_field_start_char(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_field_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn collect_field_name(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut name = String::new();
    while let Some(&c) = chars.peek() {
        if is_field_char(c) {
            name.push(c);
            chars.next();
        } else {
            break;
        }
    }
    name
}

fn parse_bracket_content(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<PathSegment> {
    match chars.peek() {
        Some(']') => {
            chars.next();
            Some(PathSegment::ArrayIterator)
        }
        Some('"') => {
            chars.next();
            let name = collect_quoted_string(chars);
            skip_closing_bracket(chars);
            Some(PathSegment::Field(name))
        }
        Some(&c) if c.is_ascii_digit() || c == '-' => {
            let index = collect_number(chars);
            skip_closing_bracket(chars);
            Some(PathSegment::ArrayIndex(index))
        }
        _ => None,
    }
}

fn collect_quoted_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut result = String::new();
    let mut escaped = false;

    for c in chars.by_ref() {
        if escaped {
            result.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            break;
        } else {
            result.push(c);
        }
    }

    result
}

fn collect_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> i64 {
    let mut num_str = String::new();

    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '-' {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }

    num_str.parse().unwrap_or(0)
}

fn skip_closing_bracket(chars: &mut std::iter::Peekable<std::str::Chars>) {
    if chars.peek() == Some(&']') {
        chars.next();
    }
}

#[cfg(test)]
#[path = "provenance_tests.rs"]
mod provenance_tests;
