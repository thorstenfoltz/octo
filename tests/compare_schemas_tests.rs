//! Tests for `octa::data::compare_schemas`. Pure function so no I/O is
//! involved. Builds `ColumnInfo` slices directly via a local helper.

use octa::data::ColumnInfo;
use octa::data::compare_schemas::{SchemaDiff, TypeMismatch, compare_schemas};

fn cols(pairs: &[(&str, &str)]) -> Vec<ColumnInfo> {
    pairs
        .iter()
        .map(|(name, ty)| ColumnInfo {
            name: name.to_string(),
            data_type: ty.to_string(),
        })
        .collect()
}

#[test]
fn identical_schemas() {
    let a = cols(&[("id", "Int64"), ("name", "Utf8")]);
    let b = cols(&[("id", "Int64"), ("name", "Utf8")]);
    let diff = compare_schemas(&a, &b);
    assert!(diff.identical);
    assert_eq!(diff.common.len(), 2);
    assert!(diff.only_in_a.is_empty());
    assert!(diff.only_in_b.is_empty());
    assert!(diff.type_mismatches.is_empty());
}

#[test]
fn columns_only_in_a() {
    let a = cols(&[("id", "Int64"), ("legacy_flag", "Boolean")]);
    let b = cols(&[("id", "Int64")]);
    let diff = compare_schemas(&a, &b);
    assert!(!diff.identical);
    assert_eq!(diff.common.len(), 1);
    assert_eq!(diff.common[0].name, "id");
    assert_eq!(diff.only_in_a.len(), 1);
    assert_eq!(diff.only_in_a[0].name, "legacy_flag");
    assert!(diff.only_in_b.is_empty());
    assert!(diff.type_mismatches.is_empty());
}

#[test]
fn columns_only_in_b() {
    let a = cols(&[("id", "Int64")]);
    let b = cols(&[("id", "Int64"), ("region", "Utf8")]);
    let diff = compare_schemas(&a, &b);
    assert!(!diff.identical);
    assert_eq!(diff.only_in_b.len(), 1);
    assert_eq!(diff.only_in_b[0].name, "region");
}

#[test]
fn type_mismatch_reported_with_both_types() {
    let a = cols(&[("amount", "Float64")]);
    let b = cols(&[("amount", "Utf8")]);
    let diff = compare_schemas(&a, &b);
    assert!(!diff.identical);
    assert_eq!(diff.type_mismatches.len(), 1);
    assert_eq!(
        diff.type_mismatches[0],
        TypeMismatch {
            name: "amount".to_string(),
            type_a: "Float64".to_string(),
            type_b: "Utf8".to_string(),
        }
    );
    // A mismatched column must NOT appear in `common`.
    assert!(diff.common.is_empty());
}

#[test]
fn case_sensitive_names_do_not_match() {
    let a = cols(&[("ID", "Int64")]);
    let b = cols(&[("id", "Int64")]);
    let diff = compare_schemas(&a, &b);
    assert!(!diff.identical);
    assert_eq!(diff.only_in_a.len(), 1);
    assert_eq!(diff.only_in_b.len(), 1);
    assert!(diff.common.is_empty());
}

#[test]
fn order_preserved_from_side_a() {
    // `common` should follow A's order, not alphabetical or B's order.
    let a = cols(&[("z", "Int64"), ("a", "Utf8"), ("m", "Boolean")]);
    let b = cols(&[("a", "Utf8"), ("m", "Boolean"), ("z", "Int64")]);
    let diff = compare_schemas(&a, &b);
    let names: Vec<&str> = diff.common.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["z", "a", "m"]);
}

#[test]
fn only_in_b_order_follows_b() {
    let a = cols(&[("id", "Int64")]);
    let b = cols(&[("z", "Int64"), ("a", "Utf8"), ("m", "Boolean")]);
    let diff = compare_schemas(&a, &b);
    let names: Vec<&str> = diff.only_in_b.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["z", "a", "m"]);
}

#[test]
fn both_empty_is_identical() {
    let diff = compare_schemas(&[], &[]);
    assert!(diff.identical);
    assert!(diff.common.is_empty());
    assert!(diff.only_in_a.is_empty());
    assert!(diff.only_in_b.is_empty());
    assert!(diff.type_mismatches.is_empty());
}

#[test]
fn one_empty_reports_only_in_other() {
    let a = cols(&[("id", "Int64"), ("name", "Utf8")]);
    let diff = compare_schemas(&a, &[]);
    assert!(!diff.identical);
    assert_eq!(diff.only_in_a.len(), 2);

    let diff_rev = compare_schemas(&[], &a);
    assert!(!diff_rev.identical);
    assert_eq!(diff_rev.only_in_b.len(), 2);
}

#[test]
fn schema_diff_is_partial_eq() {
    // Sanity: the struct derives Eq + PartialEq so tests can compare
    // whole-diff values, not just field-by-field.
    let a = cols(&[("id", "Int64")]);
    let b = cols(&[("id", "Int64")]);
    let diff1 = compare_schemas(&a, &b);
    let diff2 = SchemaDiff {
        common: cols(&[("id", "Int64")]),
        only_in_a: Vec::new(),
        only_in_b: Vec::new(),
        type_mismatches: Vec::new(),
        identical: true,
    };
    assert_eq!(diff1, diff2);
}
