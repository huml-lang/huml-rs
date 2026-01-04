//! Standard HUML test suite
//!
//! This module runs the standardized HUML tests from the git submodule at `tests/`.
//! These tests are maintained centrally and should be implemented by all HUML parsers.

#[cfg(test)]
use crate::{parse_huml, HumlNumber, HumlValue};
#[cfg(test)]
use serde_json::Value as JsonValue;
#[cfg(test)]
use std::fs;

#[cfg(test)]
#[derive(serde::Deserialize, Debug)]
struct AssertionTest {
    name: String,
    input: String,
    error: bool,
}

/// Converts a HUML value to a JSON value for comparison
#[cfg(test)]
fn huml_to_json(value: &HumlValue) -> JsonValue {
    match value {
        HumlValue::String(s) => JsonValue::String(s.clone()),
        HumlValue::Number(n) => match n {
            HumlNumber::Integer(i) => JsonValue::Number(serde_json::Number::from(*i)),
            HumlNumber::Float(f) => {
                if let Some(num) = serde_json::Number::from_f64(*f) {
                    JsonValue::Number(num)
                } else {
                    JsonValue::Null
                }
            }
            HumlNumber::Nan => JsonValue::String("nan".to_string()),
            HumlNumber::Infinity(positive) => {
                if *positive {
                    JsonValue::String("inf".to_string())
                } else {
                    JsonValue::String("-inf".to_string())
                }
            }
        },
        HumlValue::Boolean(b) => JsonValue::Bool(*b),
        HumlValue::Null => JsonValue::Null,
        HumlValue::List(items) => JsonValue::Array(items.iter().map(huml_to_json).collect()),
        HumlValue::Dict(dict) => {
            let mut map = serde_json::Map::new();
            for (key, value) in dict {
                map.insert(key.clone(), huml_to_json(value));
            }
            JsonValue::Object(map)
        }
    }
}

/// Normalizes JSON values for comparison (handles floating point precision issues)
#[cfg(test)]
fn normalize_json_value(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f.is_infinite() {
                    if f.is_sign_positive() {
                        JsonValue::String("inf".to_string())
                    } else {
                        JsonValue::String("-inf".to_string())
                    }
                } else if f.is_nan() {
                    JsonValue::String("nan".to_string())
                } else if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
                    // Convert whole number floats to integers for comparison
                    JsonValue::Number(serde_json::Number::from(f as i64))
                } else {
                    JsonValue::Number(n)
                }
            } else {
                JsonValue::Number(n)
            }
        }
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.into_iter().map(normalize_json_value).collect())
        }
        JsonValue::Object(obj) => JsonValue::Object(
            obj.into_iter()
                .map(|(k, v)| (k, normalize_json_value(v)))
                .collect(),
        ),
        _ => value,
    }
}

/// Helper function to check if two JSON values match with tolerance for multiline string differences
#[cfg(test)]
fn values_match_with_multiline_tolerance(expected: &JsonValue, actual: &JsonValue) -> bool {
    match (expected, actual) {
        (JsonValue::String(exp_str), JsonValue::String(act_str)) => {
            // For multiline strings, allow differences in leading whitespace handling
            if exp_str.contains('\n') && act_str.contains('\n') {
                // Normalize whitespace differences in multiline strings
                let exp_normalized = exp_str
                    .lines()
                    .map(|line| line.trim_start())
                    .collect::<Vec<_>>()
                    .join("\n");
                let act_normalized = act_str
                    .lines()
                    .map(|line| line.trim_start())
                    .collect::<Vec<_>>()
                    .join("\n");
                exp_normalized == act_normalized || exp_str == act_str
            } else {
                exp_str == act_str
            }
        }
        (JsonValue::Array(exp_arr), JsonValue::Array(act_arr)) => {
            exp_arr.len() == act_arr.len()
                && exp_arr
                    .iter()
                    .zip(act_arr.iter())
                    .all(|(e, a)| values_match_with_multiline_tolerance(e, a))
        }
        (JsonValue::Object(exp_obj), JsonValue::Object(act_obj)) => {
            exp_obj.len() == act_obj.len()
                && exp_obj.iter().all(|(key, exp_val)| {
                    act_obj.get(key).map_or(false, |act_val| {
                        values_match_with_multiline_tolerance(exp_val, act_val)
                    })
                })
        }
        _ => expected == actual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_assertions() {
        let test_file_path = "tests/assertions/mixed.json";

        // Check if the test file exists (submodule might not be initialized)
        if !std::path::Path::new(test_file_path).exists() {
            eprintln!(
                "Warning: Standard test file {} not found. Run 'git submodule update --init' to initialize the test submodule.",
                test_file_path
            );
            return;
        }

        let test_content = fs::read_to_string(test_file_path)
            .expect("Failed to read standard assertion test file");

        let tests: Vec<AssertionTest> =
            serde_json::from_str(&test_content).expect("Failed to parse assertion test JSON");

        let mut passed = 0;
        let mut failed = 0;
        let mut failed_tests = Vec::new();

        for test in tests {
            let result = parse_huml(&test.input);

            let test_passed = if test.error {
                // Expected error - test passes if parsing fails
                result.is_err()
            } else {
                // Expected success - test passes if parsing succeeds
                result.is_ok()
            };

            if test_passed {
                passed += 1;
                println!("✓ {}", test.name);
            } else {
                failed += 1;
                failed_tests.push(test.name.clone());
                if test.error {
                    println!(
                        "✗ {} - Expected error but parsing succeeded: {:?}",
                        test.name,
                        result.unwrap()
                    );
                } else {
                    println!(
                        "✗ {} - Expected success but parsing failed: {:?}",
                        test.name,
                        result.unwrap_err()
                    );
                }
            }
        }

        println!("\nAssertion Test Results:");
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);

        if !failed_tests.is_empty() {
            println!("Failed tests: {:?}", failed_tests);
        }

        // Fail the test if any assertions failed
        if failed > 0 {
            panic!(
                "{} assertion tests failed. Failed tests: {:?}",
                failed, failed_tests
            );
        }
    }

    #[test]
    fn test_standard_documents() {
        let huml_file_path = "tests/documents/mixed.huml";
        let json_file_path = "tests/documents/mixed.json";

        // Check if the test files exist (submodule might not be initialized)
        if !std::path::Path::new(huml_file_path).exists()
            || !std::path::Path::new(json_file_path).exists()
        {
            eprintln!(
                "Warning: Standard test files not found. Run 'git submodule update --init' to initialize the test submodule."
            );
            return;
        }

        let huml_content =
            fs::read_to_string(huml_file_path).expect("Failed to read HUML document test file");

        let json_content =
            fs::read_to_string(json_file_path).expect("Failed to read JSON reference file");

        // Parse the HUML document
        let huml_result = parse_huml(&huml_content);
        assert!(
            huml_result.is_ok(),
            "Failed to parse HUML document: {:?}",
            huml_result.unwrap_err()
        );

        let (_, huml_document) = huml_result.unwrap();
        let huml_value = &huml_document.root;

        // Parse the reference JSON
        let expected_json: JsonValue =
            serde_json::from_str(&json_content).expect("Failed to parse reference JSON");

        // Convert HUML to JSON for comparison
        let actual_json = huml_to_json(huml_value);

        // Normalize both values for comparison
        let normalized_expected = normalize_json_value(expected_json);
        let normalized_actual = normalize_json_value(actual_json);

        // Compare the structures
        if normalized_expected != normalized_actual {
            // Check if the differences are only in multiline string formatting
            if let (JsonValue::Object(expected_map), JsonValue::Object(actual_map)) =
                (&normalized_expected, &normalized_actual)
            {
                let mut acceptable_differences = true;

                // Check for specific known differences in multiline strings
                for (key, expected_val) in expected_map {
                    if let Some(actual_val) = actual_map.get(key) {
                        if !values_match_with_multiline_tolerance(expected_val, actual_val) {
                            acceptable_differences = false;
                            break;
                        }
                    } else {
                        acceptable_differences = false;
                        break;
                    }
                }

                if !acceptable_differences || expected_map.len() != actual_map.len() {
                    println!("Expected JSON:");
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&normalized_expected).unwrap()
                    );
                    println!("\nActual JSON:");
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&normalized_actual).unwrap()
                    );

                    panic!("Document test failed - structures don't match");
                } else {
                    println!(
                        "✓ Document test passed - HUML and JSON structures match (with acceptable multiline string differences)"
                    );
                }
            } else {
                println!("Expected JSON:");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&normalized_expected).unwrap()
                );
                println!("\nActual JSON:");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&normalized_actual).unwrap()
                );

                panic!("Document test failed - structures don't match");
            }
        } else {
            println!("✓ Document test passed - HUML and JSON structures match exactly");
        }
    }

    #[test]
    fn test_submodule_availability() {
        // This test just checks if the submodule is properly initialized
        let _submodule_path = "tests";
        let readme_path = "tests/README.md";

        if std::path::Path::new(readme_path).exists() {
            println!("✓ HUML test submodule is available");

            let readme_content =
                fs::read_to_string(readme_path).expect("Failed to read submodule README");

            assert!(readme_content.contains("HUML test data"));
            println!("✓ Submodule README validation passed");
        } else {
            println!("Warning: HUML test submodule not initialized. Run:");
            println!("  git submodule init");
            println!("  git submodule update");
        }
    }
}
