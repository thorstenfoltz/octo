//! Load-time whitespace normalization.
//!
//! [`trim_string_columns`] strips leading and trailing whitespace from every
//! `CellValue::String` cell in a table, in place. Interior whitespace is
//! never touched. It returns the names of the columns where at least one cell
//! actually changed, so the caller can surface a "trimmed N column(s)" notice.
//!
//! This is a normalization pass, not a tracked edit - it mutates `table.rows`
//! directly and does not push to the undo stack. The app gates it behind the
//! `trim_whitespace_on_load` setting.

use crate::data::{CellValue, DataTable};

/// Trim leading/trailing whitespace from all string cells **and column
/// titles** in `table`. Returns the (trimmed) names of columns that had their
/// title or at least one cell trimmed, in column order.
pub fn trim_string_columns(table: &mut DataTable) -> Vec<String> {
    let col_count = table.columns.len();
    let mut trimmed_cols = vec![false; col_count];

    // Column titles.
    for (col_idx, col) in table.columns.iter_mut().enumerate() {
        let trimmed = col.name.trim();
        if trimmed.len() != col.name.len() {
            col.name = trimmed.to_string();
            trimmed_cols[col_idx] = true;
        }
    }

    // Cell values.
    for row in &mut table.rows {
        for (col_idx, cell) in row.iter_mut().enumerate().take(col_count) {
            if let CellValue::String(s) = cell {
                let trimmed = s.trim();
                if trimmed.len() != s.len() {
                    *s = trimmed.to_string();
                    trimmed_cols[col_idx] = true;
                }
            }
        }
    }

    trimmed_cols
        .iter()
        .enumerate()
        .filter(|(_, changed)| **changed)
        .filter_map(|(idx, _)| table.columns.get(idx).map(|c| c.name.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ColumnInfo;

    fn table(cols: &[&str], rows: Vec<Vec<CellValue>>) -> DataTable {
        let mut t = DataTable::empty();
        t.columns = cols
            .iter()
            .map(|n| ColumnInfo {
                name: n.to_string(),
                data_type: "Utf8".to_string(),
            })
            .collect();
        t.rows = rows;
        t
    }

    #[test]
    fn trims_leading_and_trailing() {
        let mut t = table(
            &["a", "b"],
            vec![vec![
                CellValue::String("  hi  ".into()),
                CellValue::String("ok".into()),
            ]],
        );
        let changed = trim_string_columns(&mut t);
        assert_eq!(changed, vec!["a".to_string()]);
        assert_eq!(t.rows[0][0], CellValue::String("hi".into()));
        assert_eq!(t.rows[0][1], CellValue::String("ok".into()));
    }

    #[test]
    fn preserves_interior_whitespace() {
        let mut t = table(&["a"], vec![vec![CellValue::String("  a  b  ".into())]]);
        let changed = trim_string_columns(&mut t);
        assert_eq!(changed, vec!["a".to_string()]);
        assert_eq!(t.rows[0][0], CellValue::String("a  b".into()));
    }

    #[test]
    fn leaves_non_string_cells() {
        let mut t = table(
            &["n", "s"],
            vec![vec![CellValue::Int(5), CellValue::String("x".into())]],
        );
        let changed = trim_string_columns(&mut t);
        assert!(changed.is_empty());
        assert_eq!(t.rows[0][0], CellValue::Int(5));
    }

    #[test]
    fn reports_each_affected_column() {
        let mut t = table(
            &["a", "b", "c"],
            vec![
                vec![
                    CellValue::String("x ".into()),
                    CellValue::String("y".into()),
                    CellValue::String(" z".into()),
                ],
                vec![
                    CellValue::String("p".into()),
                    CellValue::String("q".into()),
                    CellValue::String("r".into()),
                ],
            ],
        );
        let changed = trim_string_columns(&mut t);
        assert_eq!(changed, vec!["a".to_string(), "c".to_string()]);
    }
}
