use octa::data::CellValue;
use octa::formats::FormatReader;
use octa::formats::jupyter_reader::JupyterReader;
use std::io::Write;

fn write_temp_notebook(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::Builder::new()
        .suffix(".ipynb")
        .tempfile()
        .unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

fn sample_notebook() -> &'static str {
    r##"{
  "nbformat": 4,
  "nbformat_minor": 5,
  "metadata": {
    "kernelspec": {
      "display_name": "Python 3",
      "language": "python",
      "name": "python3"
    }
  },
  "cells": [
    {
      "cell_type": "markdown",
      "metadata": {},
      "source": ["# Hello World\n", "This is a test notebook."]
    },
    {
      "cell_type": "code",
      "metadata": {},
      "source": ["print('hello')\n", "x = 42"],
      "execution_count": 1,
      "outputs": [
        {
          "output_type": "stream",
          "name": "stdout",
          "text": ["hello\n"]
        }
      ]
    },
    {
      "cell_type": "code",
      "metadata": {},
      "source": ["x * 2"],
      "execution_count": 2,
      "outputs": [
        {
          "output_type": "execute_result",
          "data": {
            "text/plain": ["84"]
          },
          "metadata": {},
          "execution_count": 2
        }
      ]
    },
    {
      "cell_type": "code",
      "metadata": {},
      "source": ["1/0"],
      "execution_count": 3,
      "outputs": [
        {
          "output_type": "error",
          "ename": "ZeroDivisionError",
          "evalue": "division by zero",
          "traceback": ["..."]
        }
      ]
    }
  ]
}"##
}

// --- Reader basics ---

#[test]
fn test_reader_name() {
    assert_eq!(JupyterReader.name(), "Jupyter Notebook");
}

#[test]
fn test_reader_extensions() {
    assert_eq!(JupyterReader.extensions(), &["ipynb"]);
}

#[test]
fn test_reader_supports_write() {
    assert!(JupyterReader.supports_write());
}

// --- Reading notebooks ---

#[test]
fn test_read_cell_count() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    assert_eq!(table.row_count(), 4);
    assert_eq!(table.col_count(), 4);
}

#[test]
fn test_read_column_names() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    assert_eq!(table.columns[0].name, "Cell");
    assert_eq!(table.columns[1].name, "Type");
    assert_eq!(table.columns[2].name, "Source");
    assert_eq!(table.columns[3].name, "Output");
}

#[test]
fn test_read_cell_numbers() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    assert_eq!(table.get(0, 0), Some(&CellValue::Int(1)));
    assert_eq!(table.get(1, 0), Some(&CellValue::Int(2)));
    assert_eq!(table.get(2, 0), Some(&CellValue::Int(3)));
    assert_eq!(table.get(3, 0), Some(&CellValue::Int(4)));
}

#[test]
fn test_read_cell_types() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    assert_eq!(
        table.get(0, 1),
        Some(&CellValue::String("markdown".into()))
    );
    assert_eq!(table.get(1, 1), Some(&CellValue::String("code".into())));
    assert_eq!(table.get(2, 1), Some(&CellValue::String("code".into())));
}

#[test]
fn test_read_markdown_source() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    let source = match table.get(0, 2) {
        Some(CellValue::String(s)) => s.clone(),
        _ => panic!("Expected string"),
    };
    assert!(source.contains("# Hello World"));
    assert!(source.contains("This is a test notebook."));
}

#[test]
fn test_read_code_source() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    let source = match table.get(1, 2) {
        Some(CellValue::String(s)) => s.clone(),
        _ => panic!("Expected string"),
    };
    assert!(source.contains("print('hello')"));
    assert!(source.contains("x = 42"));
}

#[test]
fn test_read_stream_output() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    let output = match table.get(1, 3) {
        Some(CellValue::String(s)) => s.clone(),
        _ => panic!("Expected string"),
    };
    assert!(output.contains("hello"));
}

#[test]
fn test_read_execute_result_output() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    let output = match table.get(2, 3) {
        Some(CellValue::String(s)) => s.clone(),
        _ => panic!("Expected string"),
    };
    assert_eq!(output, "84");
}

#[test]
fn test_read_error_output() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    let output = match table.get(3, 3) {
        Some(CellValue::String(s)) => s.clone(),
        _ => panic!("Expected string"),
    };
    assert!(output.contains("ZeroDivisionError"));
    assert!(output.contains("division by zero"));
}

#[test]
fn test_read_markdown_has_no_output() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    let output = match table.get(0, 3) {
        Some(CellValue::String(s)) => s.clone(),
        _ => panic!("Expected string"),
    };
    assert!(output.is_empty());
}

// --- Edge cases ---

#[test]
fn test_read_empty_notebook() {
    let f = write_temp_notebook(r#"{"nbformat": 4, "nbformat_minor": 5, "metadata": {}, "cells": []}"#);
    let table = JupyterReader.read_file(f.path()).unwrap();
    assert_eq!(table.row_count(), 0);
    assert_eq!(table.col_count(), 4);
}

#[test]
fn test_read_invalid_json_fails() {
    let f = write_temp_notebook("not json at all");
    assert!(JupyterReader.read_file(f.path()).is_err());
}

#[test]
fn test_read_missing_cells_fails() {
    let f = write_temp_notebook(r#"{"nbformat": 4, "metadata": {}}"#);
    assert!(JupyterReader.read_file(f.path()).is_err());
}

#[test]
fn test_read_source_as_string() {
    // Some notebooks store source as a single string instead of array
    let f = write_temp_notebook(r#"{
        "nbformat": 4, "nbformat_minor": 5, "metadata": {},
        "cells": [{"cell_type": "code", "metadata": {}, "source": "x = 1", "outputs": []}]
    }"#);
    let table = JupyterReader.read_file(f.path()).unwrap();
    assert_eq!(table.get(0, 2), Some(&CellValue::String("x = 1".into())));
}

// --- Write and round-trip ---

#[test]
fn test_write_and_read_back() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();

    let out = tempfile::Builder::new()
        .suffix(".ipynb")
        .tempfile()
        .unwrap();
    JupyterReader.write_file(out.path(), &table).unwrap();

    let table2 = JupyterReader.read_file(out.path()).unwrap();
    assert_eq!(table2.row_count(), table.row_count());
    assert_eq!(table2.col_count(), table.col_count());

    // Cell types should round-trip
    for row in 0..table.row_count() {
        assert_eq!(table2.get(row, 1), table.get(row, 1)); // Type
    }
}

#[test]
fn test_write_preserves_source() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();

    let out = tempfile::Builder::new()
        .suffix(".ipynb")
        .tempfile()
        .unwrap();
    JupyterReader.write_file(out.path(), &table).unwrap();

    let table2 = JupyterReader.read_file(out.path()).unwrap();
    // Source of code cell should match
    let src1 = table.get(1, 2).unwrap().to_string();
    let src2 = table2.get(1, 2).unwrap().to_string();
    assert_eq!(src1, src2);
}

// --- Format name ---

#[test]
fn test_format_name_set() {
    let f = write_temp_notebook(sample_notebook());
    let table = JupyterReader.read_file(f.path()).unwrap();
    assert_eq!(table.format_name, Some("Jupyter Notebook".to_string()));
}
