use super::select_array_fields_for_suggestions;
use crate::autocomplete::autocomplete_state::JsonFieldType;
use serde_json::{Value, json};
use std::sync::{Mutex, OnceLock};

const SCAN_ENV: &str = "JIQ_AUTOCOMPLETE_ARRAY_SCAN_AHEAD";

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn sample_array() -> Vec<Value> {
    vec![
        json!({ "name": "api" }),
        json!({ "name": "worker", "extra_service_key": true }),
    ]
}

fn has_key(fields: &[(String, JsonFieldType)], key: &str) -> bool {
    fields.iter().any(|(k, _)| k == key)
}

#[test]
fn missing_scan_env_falls_back_to_first_object() {
    let _guard = env_lock().lock().unwrap();
    unsafe {
        std::env::remove_var(SCAN_ENV);
    }

    let array = sample_array();
    let selected = select_array_fields_for_suggestions(&array);

    assert!(has_key(&selected, "name"));
    assert!(!has_key(&selected, "extra_service_key"));
}

#[test]
fn invalid_scan_env_falls_back_to_first_object() {
    let _guard = env_lock().lock().unwrap();
    unsafe {
        std::env::set_var(SCAN_ENV, "abc");
    }

    let array = sample_array();
    let selected = select_array_fields_for_suggestions(&array);

    assert!(has_key(&selected, "name"));
    assert!(!has_key(&selected, "extra_service_key"));
}

#[test]
fn zero_scan_env_falls_back_to_first_object() {
    let _guard = env_lock().lock().unwrap();
    unsafe {
        std::env::set_var(SCAN_ENV, "0");
    }

    let array = sample_array();
    let selected = select_array_fields_for_suggestions(&array);

    assert!(has_key(&selected, "name"));
    assert!(!has_key(&selected, "extra_service_key"));
}

#[test]
fn valid_scan_env_collects_unique_keys_within_prefix() {
    let _guard = env_lock().lock().unwrap();
    unsafe {
        std::env::set_var(SCAN_ENV, "2");
    }

    let array = sample_array();
    let selected = select_array_fields_for_suggestions(&array);

    assert!(has_key(&selected, "name"));
    assert!(has_key(&selected, "extra_service_key"));
}

#[test]
fn scan_env_one_keeps_first_object_behavior() {
    let _guard = env_lock().lock().unwrap();
    unsafe {
        std::env::set_var(SCAN_ENV, "1");
    }

    let array = sample_array();
    let selected = select_array_fields_for_suggestions(&array);

    assert!(has_key(&selected, "name"));
    assert!(!has_key(&selected, "extra_service_key"));
}
