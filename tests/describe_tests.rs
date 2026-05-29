//! Tests for `octa::data::describe::describe_file`. Uses on-disk
//! fixtures (CSV, JSON) so we exercise the registry → reader →
//! describe pipeline end-to-end.

use std::io::Write;

use octa::data::describe::{DEFAULT_SAMPLE_ROWS, MAX_SAMPLE_ROWS, describe_file};

fn write_temp_csv(text: &str) -> tempfile::NamedTempFile {
    let mut tf = tempfile::Builder::new()
        .suffix(".csv")
        .tempfile()
        .expect("tempfile");
    write!(tf.as_file_mut(), "{text}").expect("write csv");
    tf
}

#[test]
fn describes_small_csv_end_to_end() {
    let tf = write_temp_csv("a,b,c\n1,2,3\n4,5,6\n7,8,9\n");
    let d = describe_file(tf.path(), None, None).expect("describe");
    assert_eq!(d.columns.len(), 3);
    let names: Vec<_> = d.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);
    assert_eq!(d.row_count, 3);
    // Default sample size is DEFAULT_SAMPLE_ROWS (5), capped by the
    // actual row count.
    assert_eq!(d.sample_rows.len(), 3);
}

#[test]
fn sample_rows_clamps_to_actual_row_count() {
    let tf = write_temp_csv("a,b\n1,2\n");
    let d = describe_file(tf.path(), None, Some(50)).expect("describe");
    assert_eq!(d.row_count, 1);
    assert_eq!(d.sample_rows.len(), 1);
}

#[test]
fn sample_rows_clamps_to_max_constant() {
    // Build a CSV with > MAX_SAMPLE_ROWS rows; ask for 10_000.
    let mut text = String::from("a\n");
    for i in 0..(MAX_SAMPLE_ROWS + 50) {
        text.push_str(&format!("{i}\n"));
    }
    let tf = write_temp_csv(&text);
    let d = describe_file(tf.path(), None, Some(10_000)).expect("describe");
    assert!(d.row_count >= MAX_SAMPLE_ROWS);
    assert_eq!(d.sample_rows.len(), MAX_SAMPLE_ROWS);
}

#[test]
fn default_sample_size_kicks_in_when_none() {
    let mut text = String::from("a\n");
    for i in 0..20 {
        text.push_str(&format!("{i}\n"));
    }
    let tf = write_temp_csv(&text);
    let d = describe_file(tf.path(), None, None).expect("describe");
    assert_eq!(d.sample_rows.len(), DEFAULT_SAMPLE_ROWS);
}

#[test]
fn includes_file_size_and_path() {
    let tf = write_temp_csv("x\n1\n");
    let d = describe_file(tf.path(), None, None).expect("describe");
    // File exists, so file_size_bytes should be populated and > 0.
    assert!(d.file_size_bytes.is_some());
    assert!(d.file_size_bytes.unwrap() > 0);
    // Path should reflect what was passed in.
    assert_eq!(d.path, tf.path().display().to_string());
}

#[test]
fn empty_table_has_empty_sample() {
    let tf = write_temp_csv("a,b\n");
    let d = describe_file(tf.path(), None, Some(5)).expect("describe");
    assert_eq!(d.row_count, 0);
    assert!(d.sample_rows.is_empty());
    assert_eq!(d.columns.len(), 2);
}
