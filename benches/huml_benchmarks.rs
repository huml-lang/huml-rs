use criterion::{Criterion, black_box, criterion_group, criterion_main};
use huml_rs::*;
use std::fs;

fn load_test_file() -> String {
    fs::read_to_string("test.huml").expect("Failed to read test.huml")
}

fn benchmark_full_parse(c: &mut Criterion) {
    let content = load_test_file();

    c.bench_function("parse_full_huml_document", |b| {
        b.iter(|| {
            let result = parse_huml(black_box(&content));
            black_box(result)
        })
    });
}

fn benchmark_parse_components(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_components");

    // Test various scalar values
    group.bench_function("parse_string_simple", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box("\"hello world\""));
            black_box(result)
        })
    });

    group.bench_function("parse_string_complex", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box(
                "\"Hello \\\"World\\\" with 'quotes' and unicode: αβγδε 中文 🚀\"",
            ));
            black_box(result)
        })
    });

    group.bench_function("parse_integer", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box("1234567890"));
            black_box(result)
        })
    });

    group.bench_function("parse_float", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box("3.14159265359"));
            black_box(result)
        })
    });

    group.bench_function("parse_scientific_notation", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box("1.23e-10"));
            black_box(result)
        })
    });

    group.bench_function("parse_hex_number", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box("0xDEADBEEF"));
            black_box(result)
        })
    });

    group.bench_function("parse_binary_number", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box("0b1010101"));
            black_box(result)
        })
    });

    group.bench_function("parse_octal_number", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box("0o777"));
            black_box(result)
        })
    });

    group.bench_function("parse_boolean_true", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box("true"));
            black_box(result)
        })
    });

    group.bench_function("parse_null", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box("null"));
            black_box(result)
        })
    });

    group.finish();
}

fn benchmark_collections(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_collections");

    // Empty collections
    group.bench_function("parse_empty_list", |b| {
        b.iter(|| {
            let result = parse_empty_list(black_box("[]"));
            black_box(result)
        })
    });

    group.bench_function("parse_empty_dict", |b| {
        b.iter(|| {
            let result = parse_empty_dict(black_box("{}"));
            black_box(result)
        })
    });

    // Inline collections
    group.bench_function("parse_inline_list_simple", |b| {
        b.iter(|| {
            let result = parse_inline_list(black_box("1, 2, 3, 4, 5"));
            black_box(result)
        })
    });

    group.bench_function("parse_inline_list_mixed", |b| {
        b.iter(|| {
            let result = parse_inline_list(black_box("\"string\", 42, true, null, 3.14"));
            black_box(result)
        })
    });

    group.bench_function("parse_inline_list_large", |b| {
        b.iter(|| {
            let result = parse_inline_list(black_box(
                "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, \"eleven\", \"twelve\", true, false, null, 3.14",
            ));
            black_box(result)
        })
    });

    group.bench_function("parse_inline_dict_simple", |b| {
        b.iter(|| {
            let result = parse_inline_dict(black_box("foo: \"bar\", baz: 123"));
            black_box(result)
        })
    });

    group.bench_function("parse_inline_dict_complex", |b| {
        b.iter(|| {
            let result = parse_inline_dict(black_box(
                "a: 1, b: 2, c: 3, d: 4, e: 5, f: \"six\", g: true, h: null",
            ));
            black_box(result)
        })
    });

    group.finish();
}

fn benchmark_multiline_strings(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_multiline_strings");

    let preserved_string = r#"```
    Preserved formatting
      With different indentation
        And multiple levels
      Back to level two
    Back to level one
  ```"#;

    let stripped_string = r#""""
    Stripped formatting
      This will be normalized
        All leading whitespace removed
      Consistent indentation
    Final line
  """"#;

    group.bench_function("parse_multiline_preserved", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box(preserved_string));
            black_box(result)
        })
    });

    group.bench_function("parse_multiline_stripped", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box(stripped_string));
            black_box(result)
        })
    });

    group.finish();
}

fn benchmark_document_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_documents");

    // Simple document
    let simple_doc = r#"# Simple document
key: "value"
number: 42
flag: true"#;

    // Complex nested document
    let complex_doc = r#"# Complex document
section::
  nested::
    deep::
      value: "deep_value"
      list::
        - "item1"
        - :: key: "value"
        - "item3"
      inline_dict:: a: 1, b: 2, c: 3
  strings::
    simple: "hello"
    complex: "Hello \"World\" with unicode: 中文 🚀"
    multiline: ```
      Preserved
        formatting
      here
    ```
  numbers::
    integer: 1234567
    float: 3.14159
    scientific: 1.23e-10
    hex: 0xDEADBEEF
    binary: 0b1010101
    octal: 0o777
collections::
  list:: "a", "b", "c", true, false, null, 42, 3.14
  dict:: key1: "value1", key2: 42, key3: true"#;

    group.bench_function("parse_simple_document", |b| {
        b.iter(|| {
            let result = parse_huml(black_box(simple_doc));
            black_box(result)
        })
    });

    group.bench_function("parse_complex_document", |b| {
        b.iter(|| {
            let result = parse_huml(black_box(complex_doc));
            black_box(result)
        })
    });

    group.finish();
}

fn benchmark_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_edge_cases");

    // Very long strings
    let long_string = format!("\"{}\"", "a".repeat(1000));
    group.bench_function("parse_long_string", |b| {
        b.iter(|| {
            let result = parse_scalar(black_box(&long_string));
            black_box(result)
        })
    });

    // Deep nesting
    let deep_nested = r#"# Deep nesting
level1::
  level2::
    level3::
      level4::
        level5::
          level6::
            level7::
              level8::
                level9::
                  level10::
                    deep_value: "very_deep"
                    deep_list::
                      - "item1"
                      - :: nested: "dict"
                      - "item3""#;

    group.bench_function("parse_deep_nesting", |b| {
        b.iter(|| {
            let result = parse_huml(black_box(deep_nested));
            black_box(result)
        })
    });

    // Large inline collections
    let large_list: Vec<String> = (1..=100).map(|i| i.to_string()).collect();
    let large_list_str = large_list.join(", ");

    group.bench_function("parse_large_inline_list", |b| {
        b.iter(|| {
            let result = parse_inline_list(black_box(&large_list_str));
            black_box(result)
        })
    });

    // Many dict entries
    let large_dict: Vec<String> = (1..=50)
        .map(|i| format!("key{}: \"value{}\"", i, i))
        .collect();
    let large_dict_str = large_dict.join(", ");

    group.bench_function("parse_large_inline_dict", |b| {
        b.iter(|| {
            let result = parse_inline_dict(black_box(&large_dict_str));
            black_box(result)
        })
    });

    // Stress test with many comments
    let commented_doc = r#"# Main document with many comments
# This is a comment
section:: # Another comment
  # Yet another comment
  key: "value" # End of line comment
  # More comments
  nested:: # Nested comment
    # Deep comment
    deep_key: "deep_value" # Final comment
    # Last comment
"#;

    group.bench_function("parse_many_comments", |b| {
        b.iter(|| {
            let result = parse_huml(black_box(commented_doc));
            black_box(result)
        })
    });

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    // Configure to run fewer iterations for memory tests
    group.sample_size(10);

    let content = load_test_file();

    group.bench_function("full_document_memory", |b| {
        b.iter(|| {
            let result = parse_huml(black_box(&content));
            // Force allocation and deallocation
            if let Ok((remaining, ref doc)) = result {
                let _ = format!("{:?}", doc);
                black_box((remaining, doc));
            }
            black_box(result)
        })
    });

    // Test repeated parsing of same content
    group.bench_function("repeated_parsing", |b| {
        b.iter(|| {
            for _ in 0..10 {
                let result = parse_huml(black_box(&content));
                let _ = black_box(result);
            }
        })
    });

    group.finish();
}

fn benchmark_different_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("different_sizes");

    // Small document
    let small_doc = r#"# Small
key: "value"
number: 42"#;

    // Medium document
    let medium_doc = r#"# Medium document
section1::
  key1: "value1"
  key2: 42
  key3: true
  nested::
    deep_key: "deep_value"
    list:: 1, 2, 3, 4, 5
section2::
  more_keys: "more_values"
  another_list::
    - "item1"
    - "item2"
    - :: nested: "dict"
section3::
  final_section: "done""#;

    // Large document (programmatically generated)
    let mut large_sections = Vec::new();
    for i in 1..=20 {
        large_sections.push(format!(
            r#"section{}::
  key{}: "value{}"
  number{}: {}
  bool{}: {}
  nested{}::
    deep_key{}: "deep_value{}"
    list{}: {}"#,
            i,
            i,
            i,
            i,
            i * 10,
            i,
            i % 2 == 0,
            i,
            i,
            i,
            i,
            (1..=10)
                .map(|j| (i * 10 + j).to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let large_doc = format!("# Large document\n{}", large_sections.join("\n"));

    group.bench_function("small_document", |b| {
        b.iter(|| {
            let result = parse_huml(black_box(small_doc));
            black_box(result)
        })
    });

    group.bench_function("medium_document", |b| {
        b.iter(|| {
            let result = parse_huml(black_box(medium_doc));
            black_box(result)
        })
    });

    group.bench_function("large_document", |b| {
        b.iter(|| {
            let result = parse_huml(black_box(&large_doc));
            black_box(result)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_full_parse,
    benchmark_parse_components,
    benchmark_collections,
    benchmark_multiline_strings,
    benchmark_document_parsing,
    benchmark_edge_cases,
    benchmark_memory_usage,
    benchmark_different_sizes
);
criterion_main!(benches);
