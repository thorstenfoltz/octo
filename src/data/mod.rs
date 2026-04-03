use std::collections::HashMap;
use std::fmt;

/// How to display the loaded file content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Structured tabular view (default).
    Table,
    /// Raw text view of the file content (like a text editor).
    Raw,
    /// Rendered PDF page view (like Adobe Reader).
    Pdf,
}

/// Represents a single cell value in the data table.
/// Supports structured (typed columns) and semi-structured (mixed types) data.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Date(String),
    DateTime(String),
    Binary(Vec<u8>),
    /// For semi-structured nested data (JSON objects, arrays, etc.)
    Nested(String),
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellValue::Null => write!(f, ""),
            CellValue::Bool(b) => write!(f, "{}", b),
            CellValue::Int(i) => write!(f, "{}", i),
            CellValue::Float(v) => {
                if v.fract() == 0.0 && v.abs() < 1e15 {
                    write!(f, "{:.1}", v)
                } else {
                    write!(f, "{}", v)
                }
            }
            CellValue::String(s) => write!(f, "{}", s),
            CellValue::Date(s) => write!(f, "{}", s),
            CellValue::DateTime(s) => write!(f, "{}", s),
            CellValue::Binary(b) => write!(f, "<{} bytes>", b.len()),
            CellValue::Nested(s) => write!(f, "{}", s),
        }
    }
}

impl CellValue {
    /// Try to parse a display string back into a CellValue, keeping the same variant
    /// as the `hint` when possible.
    pub fn parse_like(hint: &CellValue, text: &str) -> CellValue {
        if text.is_empty() {
            return CellValue::Null;
        }
        match hint {
            CellValue::Bool(_) => match text.to_lowercase().as_str() {
                "true" | "1" | "yes" => CellValue::Bool(true),
                "false" | "0" | "no" => CellValue::Bool(false),
                _ => CellValue::String(text.to_string()),
            },
            CellValue::Int(_) => text
                .parse::<i64>()
                .map(CellValue::Int)
                .unwrap_or_else(|_| CellValue::String(text.to_string())),
            CellValue::Float(_) => text
                .parse::<f64>()
                .map(CellValue::Float)
                .unwrap_or_else(|_| CellValue::String(text.to_string())),
            _ => CellValue::String(text.to_string()),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            CellValue::Null => "null",
            CellValue::Bool(_) => "bool",
            CellValue::Int(_) => "int",
            CellValue::Float(_) => "float",
            CellValue::String(_) => "string",
            CellValue::Date(_) => "date",
            CellValue::DateTime(_) => "datetime",
            CellValue::Binary(_) => "binary",
            CellValue::Nested(_) => "nested",
        }
    }
}

/// Column metadata
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
}

/// The core data model: an unbounded table of cells.
/// Rows and columns are stored as a flat Vec-of-Vecs (row-major).
/// Edits are tracked separately so the original data is preserved.
#[derive(Debug, Clone)]
pub struct DataTable {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<CellValue>>,
    /// Tracks edited cells: (row, col) -> new value
    pub edits: HashMap<(usize, usize), CellValue>,
    /// Source file path (if any)
    pub source_path: Option<String>,
    /// Format name that produced this table
    pub format_name: Option<String>,
    /// Whether structural changes have been made (add/delete/move rows/cols)
    pub structural_changes: bool,
    /// Total row count in the source file (when loading was truncated)
    pub total_rows: Option<usize>,
    /// File-level index of the first loaded row (for windowed loading)
    pub row_offset: usize,
}

impl DataTable {
    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            edits: HashMap::new(),
            source_path: None,
            format_name: None,
            structural_changes: false,
            total_rows: None,
            row_offset: 0,
        }
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn col_count(&self) -> usize {
        self.columns.len()
    }

    /// Get a cell value, respecting edits.
    pub fn get(&self, row: usize, col: usize) -> Option<&CellValue> {
        if let Some(edited) = self.edits.get(&(row, col)) {
            return Some(edited);
        }
        self.rows.get(row).and_then(|r| r.get(col))
    }

    /// Set a cell value (tracked as an edit).
    pub fn set(&mut self, row: usize, col: usize, value: CellValue) {
        // Ensure the row exists
        if row < self.rows.len() && col < self.columns.len() {
            self.edits.insert((row, col), value);
        }
    }

    /// Check if a cell has been edited.
    pub fn is_edited(&self, row: usize, col: usize) -> bool {
        self.edits.contains_key(&(row, col))
    }

    /// Discard all edits.
    pub fn discard_edits(&mut self) {
        self.edits.clear();
    }

    /// Insert a new empty row at the given index.
    /// If index >= row_count, appends at the end.
    pub fn insert_row(&mut self, index: usize) {
        self.structural_changes = true;
        let row = vec![CellValue::Null; self.columns.len()];
        let idx = index.min(self.rows.len());
        self.rows.insert(idx, row);
        // Shift edits at or after the insertion point down by 1
        let mut new_edits = HashMap::new();
        for (&(r, c), v) in &self.edits {
            if r < idx {
                new_edits.insert((r, c), v.clone());
            } else {
                new_edits.insert((r + 1, c), v.clone());
            }
        }
        self.edits = new_edits;
    }

    /// Delete a row by index.
    pub fn delete_row(&mut self, index: usize) {
        self.structural_changes = true;
        if index < self.rows.len() {
            self.rows.remove(index);
            // Clean up edits referencing this row or higher
            let mut new_edits = HashMap::new();
            for (&(r, c), v) in &self.edits {
                if r < index {
                    new_edits.insert((r, c), v.clone());
                } else if r > index {
                    new_edits.insert((r - 1, c), v.clone());
                }
                // r == index: dropped
            }
            self.edits = new_edits;
        }
    }

    /// Insert a new column at the given index with a given name and data type.
    /// If index >= col_count, appends at the end.
    pub fn insert_column(&mut self, index: usize, name: String, data_type: String) {
        self.structural_changes = true;
        let idx = index.min(self.columns.len());
        self.columns.insert(idx, ColumnInfo { name, data_type });
        for row in &mut self.rows {
            row.insert(idx, CellValue::Null);
        }
        // Shift edits at or after the insertion point right by 1
        let mut new_edits = HashMap::new();
        for (&(r, c), v) in &self.edits {
            if c < idx {
                new_edits.insert((r, c), v.clone());
            } else {
                new_edits.insert((r, c + 1), v.clone());
            }
        }
        self.edits = new_edits;
    }

    /// Delete a column by index.
    pub fn delete_column(&mut self, col_idx: usize) {
        self.structural_changes = true;
        if col_idx < self.columns.len() {
            self.columns.remove(col_idx);
            for row in &mut self.rows {
                if col_idx < row.len() {
                    row.remove(col_idx);
                }
            }
            // Clean up edits: remove edits for the deleted column, shift higher columns down
            let mut new_edits = HashMap::new();
            for (&(r, c), v) in &self.edits {
                if c < col_idx {
                    new_edits.insert((r, c), v.clone());
                } else if c > col_idx {
                    new_edits.insert((r, c - 1), v.clone());
                }
                // c == col_idx: dropped
            }
            self.edits = new_edits;
        }
    }

    /// Move a row from `from` to `to`. Both must be valid indices.
    pub fn move_row(&mut self, from: usize, to: usize) {
        if from == to || from >= self.rows.len() || to >= self.rows.len() {
            return;
        }
        self.structural_changes = true;
        let row = self.rows.remove(from);
        self.rows.insert(to, row);
        // Remap edits
        let mut new_edits = HashMap::new();
        for (&(r, c), v) in &self.edits {
            let new_r = if r == from {
                to
            } else if from < to {
                // Row moved down: rows in (from, to] shift up by 1
                if r > from && r <= to {
                    r - 1
                } else {
                    r
                }
            } else {
                // Row moved up: rows in [to, from) shift down by 1
                if r >= to && r < from {
                    r + 1
                } else {
                    r
                }
            };
            new_edits.insert((new_r, c), v.clone());
        }
        self.edits = new_edits;
    }

    /// Move a column from `from` to `to`. Both must be valid indices.
    pub fn move_column(&mut self, from: usize, to: usize) {
        if from == to || from >= self.columns.len() || to >= self.columns.len() {
            return;
        }
        self.structural_changes = true;
        let col_info = self.columns.remove(from);
        self.columns.insert(to, col_info);
        for row in &mut self.rows {
            if from < row.len() {
                let val = row.remove(from);
                let ins = to.min(row.len());
                row.insert(ins, val);
            }
        }
        // Remap edits
        let mut new_edits = HashMap::new();
        for (&(r, c), v) in &self.edits {
            let new_c = if c == from {
                to
            } else if from < to {
                if c > from && c <= to {
                    c - 1
                } else {
                    c
                }
            } else {
                if c >= to && c < from {
                    c + 1
                } else {
                    c
                }
            };
            new_edits.insert((r, new_c), v.clone());
        }
        self.edits = new_edits;
    }

    /// Reorder all columns according to a permutation.
    /// `order[new_pos] = old_pos` — i.e. the column that was at `old_pos` moves to `new_pos`.
    /// The `order` slice must be a valid permutation of `0..col_count`.
    pub fn reorder_columns(&mut self, order: &[usize]) {
        let n = self.columns.len();
        if order.len() != n {
            return;
        }
        self.structural_changes = true;

        // Reorder column metadata
        let old_cols = self.columns.clone();
        for (new_pos, &old_pos) in order.iter().enumerate() {
            self.columns[new_pos] = old_cols[old_pos].clone();
        }

        // Reorder each row's cell data
        for row in &mut self.rows {
            let old_row = row.clone();
            for (new_pos, &old_pos) in order.iter().enumerate() {
                row[new_pos] = old_row[old_pos].clone();
            }
        }

        // Remap edits: build reverse mapping (old_pos -> new_pos)
        let mut old_to_new = vec![0usize; n];
        for (new_pos, &old_pos) in order.iter().enumerate() {
            old_to_new[old_pos] = new_pos;
        }
        let mut new_edits = HashMap::new();
        for (&(r, c), v) in &self.edits {
            if c < n {
                new_edits.insert((r, old_to_new[c]), v.clone());
            }
        }
        self.edits = new_edits;
    }

    /// Sort all rows by the values in the given column, ascending or descending.
    /// Edits are applied first so sorting uses the current visible values.
    pub fn sort_rows_by_column(&mut self, col_idx: usize, ascending: bool) {
        if col_idx >= self.columns.len() || self.rows.is_empty() {
            return;
        }
        // Merge pending edits into rows first so we sort on the actual visible values
        self.apply_edits();
        self.structural_changes = true;

        self.rows.sort_by(|a, b| {
            let va = a.get(col_idx).unwrap_or(&CellValue::Null);
            let vb = b.get(col_idx).unwrap_or(&CellValue::Null);
            let cmp = cmp_cell_values(va, vb);
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }

    /// Apply all edits to the underlying data (merges edits into rows).
    /// Call this before saving to produce a clean DataTable.
    pub fn apply_edits(&mut self) {
        for (&(r, c), v) in &self.edits {
            if r < self.rows.len() && c < self.columns.len() {
                self.rows[r][c] = v.clone();
            }
        }
        self.edits.clear();
    }

    /// Whether the table has been modified in any way since loading/saving.
    pub fn is_modified(&self) -> bool {
        !self.edits.is_empty() || self.structural_changes
    }

    /// Check if all values in a column can be converted to the target data type.
    /// Returns true if the conversion is safe (all non-null values are compatible).
    pub fn can_convert_column(&self, col_idx: usize, target_type: &str) -> bool {
        if col_idx >= self.columns.len() {
            return false;
        }
        for row_idx in 0..self.rows.len() {
            let val = self.get(row_idx, col_idx).unwrap_or(&CellValue::Null);
            if !can_convert_value(val, target_type) {
                return false;
            }
        }
        true
    }

    /// Evict the first `count` rows from the table, incrementing row_offset.
    /// Remaps edits: subtracts `count` from row indices, discards edits in evicted range.
    pub fn evict_front_rows(&mut self, count: usize) {
        let count = count.min(self.rows.len());
        if count == 0 {
            return;
        }
        self.rows.drain(..count);
        self.row_offset += count;
        let mut new_edits = HashMap::new();
        for (&(r, c), v) in &self.edits {
            if r >= count {
                new_edits.insert((r - count, c), v.clone());
            }
            // Edits in evicted range (r < count) are discarded
        }
        self.edits = new_edits;
    }

    /// Reset the modification tracking (call after saving).
    pub fn clear_modified(&mut self) {
        self.structural_changes = false;
        // edits are already cleared by apply_edits
    }
}

/// Check if a single CellValue can be converted to the target data type.
fn can_convert_value(val: &CellValue, target_type: &str) -> bool {
    match val {
        CellValue::Null => true, // Null converts to anything
        CellValue::Bool(_) => match target_type {
            "String" | "Utf8" | "Boolean" | "Int64" | "Float64" => true,
            _ => false,
        },
        CellValue::Int(_) => match target_type {
            "String" | "Utf8" | "Int64" | "Float64" | "Boolean" => true,
            _ => false,
        },
        CellValue::Float(f) => match target_type {
            "String" | "Utf8" | "Float64" => true,
            "Int64" => f.fract() == 0.0 && f.abs() < i64::MAX as f64,
            _ => false,
        },
        CellValue::String(s) => {
            if s.is_empty() {
                return true;
            }
            match target_type {
                "String" | "Utf8" => true,
                "Int64" => s.parse::<i64>().is_ok(),
                "Float64" => s.parse::<f64>().is_ok(),
                "Boolean" => matches!(
                    s.to_lowercase().as_str(),
                    "true" | "false" | "1" | "0" | "yes" | "no"
                ),
                "Date32" => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok(),
                "Timestamp(Microsecond, None)" => {
                    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").is_ok()
                        || chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok()
                        || chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.3f")
                            .is_ok()
                }
                _ => false,
            }
        }
        CellValue::Date(_) => match target_type {
            "String" | "Utf8" | "Date32" | "Timestamp(Microsecond, None)" => true,
            _ => false,
        },
        CellValue::DateTime(_) => match target_type {
            "String" | "Utf8" | "Timestamp(Microsecond, None)" => true,
            "Date32" => true, // truncate time portion
            _ => false,
        },
        CellValue::Binary(_) => matches!(target_type, "String" | "Utf8"),
        CellValue::Nested(_) => matches!(target_type, "String" | "Utf8"),
    }
}

/// Compare two CellValues for sorting.
/// Ordering: Null < Bool < Int/Float (numeric) < String/Date/DateTime < Binary < Nested
fn cmp_cell_values(a: &CellValue, b: &CellValue) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (a, b) {
        (CellValue::Null, CellValue::Null) => Ordering::Equal,
        (CellValue::Null, _) => Ordering::Less,
        (_, CellValue::Null) => Ordering::Greater,

        (CellValue::Bool(a), CellValue::Bool(b)) => a.cmp(b),

        (CellValue::Int(a), CellValue::Int(b)) => a.cmp(b),
        (CellValue::Float(a), CellValue::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (CellValue::Int(a), CellValue::Float(b)) => {
            (*a as f64).partial_cmp(b).unwrap_or(Ordering::Equal)
        }
        (CellValue::Float(a), CellValue::Int(b)) => {
            a.partial_cmp(&(*b as f64)).unwrap_or(Ordering::Equal)
        }

        (CellValue::String(a), CellValue::String(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
        (CellValue::Date(a), CellValue::Date(b)) => a.cmp(b),
        (CellValue::DateTime(a), CellValue::DateTime(b)) => a.cmp(b),

        // Fallback: compare display strings
        _ => a
            .to_string()
            .to_lowercase()
            .cmp(&b.to_string().to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> DataTable {
        DataTable {
            columns: vec![
                ColumnInfo { name: "id".into(), data_type: "Int64".into() },
                ColumnInfo { name: "name".into(), data_type: "Utf8".into() },
                ColumnInfo { name: "score".into(), data_type: "Float64".into() },
            ],
            rows: vec![
                vec![CellValue::Int(1), CellValue::String("Alice".into()), CellValue::Float(9.5)],
                vec![CellValue::Int(2), CellValue::String("Bob".into()), CellValue::Float(7.0)],
                vec![CellValue::Int(3), CellValue::String("Charlie".into()), CellValue::Float(8.2)],
            ],
            edits: HashMap::new(),
            source_path: None,
            format_name: None,
            structural_changes: false,
            total_rows: None,
            row_offset: 0,
        }
    }

    // --- CellValue Display ---

    #[test]
    fn test_cell_value_display_null() {
        assert_eq!(CellValue::Null.to_string(), "");
    }

    #[test]
    fn test_cell_value_display_bool() {
        assert_eq!(CellValue::Bool(true).to_string(), "true");
        assert_eq!(CellValue::Bool(false).to_string(), "false");
    }

    #[test]
    fn test_cell_value_display_int() {
        assert_eq!(CellValue::Int(42).to_string(), "42");
        assert_eq!(CellValue::Int(-1).to_string(), "-1");
    }

    #[test]
    fn test_cell_value_display_float() {
        // Whole float gets one decimal
        assert_eq!(CellValue::Float(3.0).to_string(), "3.0");
        // Fractional float shows naturally
        assert_eq!(CellValue::Float(3.14).to_string(), "3.14");
    }

    #[test]
    fn test_cell_value_display_string() {
        assert_eq!(CellValue::String("hello".into()).to_string(), "hello");
    }

    #[test]
    fn test_cell_value_display_binary() {
        assert_eq!(CellValue::Binary(vec![1, 2, 3]).to_string(), "<3 bytes>");
    }

    // --- CellValue::parse_like ---

    #[test]
    fn test_parse_like_empty_is_null() {
        assert_eq!(CellValue::parse_like(&CellValue::Int(0), ""), CellValue::Null);
    }

    #[test]
    fn test_parse_like_bool() {
        assert_eq!(CellValue::parse_like(&CellValue::Bool(false), "true"), CellValue::Bool(true));
        assert_eq!(CellValue::parse_like(&CellValue::Bool(true), "yes"), CellValue::Bool(true));
        assert_eq!(CellValue::parse_like(&CellValue::Bool(true), "0"), CellValue::Bool(false));
        assert_eq!(CellValue::parse_like(&CellValue::Bool(true), "no"), CellValue::Bool(false));
        // Unparseable bool hint falls back to string
        assert_eq!(
            CellValue::parse_like(&CellValue::Bool(true), "maybe"),
            CellValue::String("maybe".into())
        );
    }

    #[test]
    fn test_parse_like_int() {
        assert_eq!(CellValue::parse_like(&CellValue::Int(0), "42"), CellValue::Int(42));
        assert_eq!(
            CellValue::parse_like(&CellValue::Int(0), "abc"),
            CellValue::String("abc".into())
        );
    }

    #[test]
    fn test_parse_like_float() {
        assert_eq!(CellValue::parse_like(&CellValue::Float(0.0), "3.14"), CellValue::Float(3.14));
        assert_eq!(
            CellValue::parse_like(&CellValue::Float(0.0), "xyz"),
            CellValue::String("xyz".into())
        );
    }

    #[test]
    fn test_parse_like_string_hint() {
        assert_eq!(
            CellValue::parse_like(&CellValue::String("".into()), "42"),
            CellValue::String("42".into())
        );
    }

    // --- CellValue::type_name ---

    #[test]
    fn test_type_name() {
        assert_eq!(CellValue::Null.type_name(), "null");
        assert_eq!(CellValue::Bool(true).type_name(), "bool");
        assert_eq!(CellValue::Int(1).type_name(), "int");
        assert_eq!(CellValue::Float(1.0).type_name(), "float");
        assert_eq!(CellValue::String("x".into()).type_name(), "string");
        assert_eq!(CellValue::Date("2024-01-01".into()).type_name(), "date");
        assert_eq!(CellValue::DateTime("2024-01-01T00:00:00".into()).type_name(), "datetime");
        assert_eq!(CellValue::Binary(vec![]).type_name(), "binary");
        assert_eq!(CellValue::Nested("{}".into()).type_name(), "nested");
    }

    // --- DataTable basics ---

    #[test]
    fn test_empty_table() {
        let t = DataTable::empty();
        assert_eq!(t.row_count(), 0);
        assert_eq!(t.col_count(), 0);
        assert!(!t.is_modified());
    }

    #[test]
    fn test_row_col_count() {
        let t = sample_table();
        assert_eq!(t.row_count(), 3);
        assert_eq!(t.col_count(), 3);
    }

    #[test]
    fn test_get_returns_original_value() {
        let t = sample_table();
        assert_eq!(t.get(0, 0), Some(&CellValue::Int(1)));
        assert_eq!(t.get(1, 1), Some(&CellValue::String("Bob".into())));
    }

    #[test]
    fn test_get_out_of_bounds() {
        let t = sample_table();
        assert_eq!(t.get(99, 0), None);
        assert_eq!(t.get(0, 99), None);
    }

    // --- Edit overlay ---

    #[test]
    fn test_set_creates_edit() {
        let mut t = sample_table();
        t.set(0, 1, CellValue::String("Alicia".into()));
        assert!(t.is_edited(0, 1));
        assert_eq!(t.get(0, 1), Some(&CellValue::String("Alicia".into())));
        // Original data unchanged
        assert_eq!(t.rows[0][1], CellValue::String("Alice".into()));
    }

    #[test]
    fn test_set_out_of_bounds_ignored() {
        let mut t = sample_table();
        t.set(99, 0, CellValue::Int(999));
        assert!(!t.is_edited(99, 0));
    }

    #[test]
    fn test_discard_edits() {
        let mut t = sample_table();
        t.set(0, 0, CellValue::Int(100));
        assert!(t.is_modified());
        t.discard_edits();
        assert!(!t.is_modified());
        assert_eq!(t.get(0, 0), Some(&CellValue::Int(1)));
    }

    #[test]
    fn test_apply_edits() {
        let mut t = sample_table();
        t.set(1, 1, CellValue::String("Bobby".into()));
        t.apply_edits();
        assert!(t.edits.is_empty());
        assert_eq!(t.rows[1][1], CellValue::String("Bobby".into()));
    }

    // --- Row operations ---

    #[test]
    fn test_insert_row() {
        let mut t = sample_table();
        t.insert_row(1);
        assert_eq!(t.row_count(), 4);
        assert_eq!(t.get(1, 0), Some(&CellValue::Null));
        assert_eq!(t.get(2, 1), Some(&CellValue::String("Bob".into())));
        assert!(t.structural_changes);
    }

    #[test]
    fn test_insert_row_shifts_edits() {
        let mut t = sample_table();
        t.set(1, 0, CellValue::Int(20));
        t.set(2, 0, CellValue::Int(30));
        t.insert_row(1);
        // Edit at row 1 should now be at row 2
        assert_eq!(t.get(2, 0), Some(&CellValue::Int(20)));
        // Edit at row 2 should now be at row 3
        assert_eq!(t.get(3, 0), Some(&CellValue::Int(30)));
        // Edit below insertion should be unchanged
        assert!(!t.is_edited(0, 0));
    }

    #[test]
    fn test_insert_row_at_end() {
        let mut t = sample_table();
        t.insert_row(100); // beyond end
        assert_eq!(t.row_count(), 4);
        assert_eq!(t.get(3, 0), Some(&CellValue::Null));
    }

    #[test]
    fn test_delete_row() {
        let mut t = sample_table();
        t.delete_row(1);
        assert_eq!(t.row_count(), 2);
        assert_eq!(t.get(0, 1), Some(&CellValue::String("Alice".into())));
        assert_eq!(t.get(1, 1), Some(&CellValue::String("Charlie".into())));
    }

    #[test]
    fn test_delete_row_removes_edits() {
        let mut t = sample_table();
        t.set(1, 0, CellValue::Int(99));
        t.delete_row(1);
        // Edit at deleted row is gone
        assert!(!t.is_edited(1, 0));
    }

    #[test]
    fn test_delete_row_shifts_edits_down() {
        let mut t = sample_table();
        t.set(2, 0, CellValue::Int(99));
        t.delete_row(0);
        // Edit at row 2 should now be at row 1
        assert_eq!(t.get(1, 0), Some(&CellValue::Int(99)));
    }

    // --- Column operations ---

    #[test]
    fn test_insert_column() {
        let mut t = sample_table();
        t.insert_column(1, "middle".into(), "Utf8".into());
        assert_eq!(t.col_count(), 4);
        assert_eq!(t.columns[1].name, "middle");
        assert_eq!(t.get(0, 1), Some(&CellValue::Null));
        assert_eq!(t.get(0, 2), Some(&CellValue::String("Alice".into())));
    }

    #[test]
    fn test_insert_column_shifts_edits() {
        let mut t = sample_table();
        t.set(0, 1, CellValue::String("edited".into()));
        t.insert_column(1, "new".into(), "Utf8".into());
        // Edit at col 1 should now be at col 2
        assert_eq!(t.get(0, 2), Some(&CellValue::String("edited".into())));
        assert!(!t.is_edited(0, 1));
    }

    #[test]
    fn test_delete_column() {
        let mut t = sample_table();
        t.delete_column(1); // delete "name"
        assert_eq!(t.col_count(), 2);
        assert_eq!(t.columns[0].name, "id");
        assert_eq!(t.columns[1].name, "score");
        assert_eq!(t.get(0, 1), Some(&CellValue::Float(9.5)));
    }

    #[test]
    fn test_delete_column_shifts_edits() {
        let mut t = sample_table();
        t.set(0, 2, CellValue::Float(10.0));
        t.delete_column(1);
        // Edit at col 2 should now be at col 1
        assert_eq!(t.get(0, 1), Some(&CellValue::Float(10.0)));
    }

    #[test]
    fn test_delete_column_removes_edits() {
        let mut t = sample_table();
        t.set(0, 1, CellValue::String("edited".into()));
        t.delete_column(1);
        assert!(!t.is_edited(0, 1));
    }

    // --- Move operations ---

    #[test]
    fn test_move_row_down() {
        let mut t = sample_table();
        t.move_row(0, 2);
        assert_eq!(t.get(0, 1), Some(&CellValue::String("Bob".into())));
        assert_eq!(t.get(1, 1), Some(&CellValue::String("Charlie".into())));
        assert_eq!(t.get(2, 1), Some(&CellValue::String("Alice".into())));
    }

    #[test]
    fn test_move_row_up() {
        let mut t = sample_table();
        t.move_row(2, 0);
        assert_eq!(t.get(0, 1), Some(&CellValue::String("Charlie".into())));
        assert_eq!(t.get(1, 1), Some(&CellValue::String("Alice".into())));
        assert_eq!(t.get(2, 1), Some(&CellValue::String("Bob".into())));
    }

    #[test]
    fn test_move_row_noop() {
        let mut t = sample_table();
        t.move_row(1, 1);
        assert!(!t.structural_changes);
    }

    #[test]
    fn test_move_row_remaps_edits() {
        let mut t = sample_table();
        t.set(0, 0, CellValue::Int(100));
        t.move_row(0, 2);
        // Edit should follow the moved row
        assert_eq!(t.get(2, 0), Some(&CellValue::Int(100)));
    }

    #[test]
    fn test_move_column() {
        let mut t = sample_table();
        t.move_column(0, 2); // move "id" to end
        assert_eq!(t.columns[0].name, "name");
        assert_eq!(t.columns[1].name, "score");
        assert_eq!(t.columns[2].name, "id");
        // Data should follow
        assert_eq!(t.get(0, 0), Some(&CellValue::String("Alice".into())));
        assert_eq!(t.get(0, 2), Some(&CellValue::Int(1)));
    }

    #[test]
    fn test_move_column_remaps_edits() {
        let mut t = sample_table();
        t.set(0, 0, CellValue::Int(100));
        t.move_column(0, 2);
        assert_eq!(t.get(0, 2), Some(&CellValue::Int(100)));
    }

    // --- Reorder columns ---

    #[test]
    fn test_reorder_columns() {
        let mut t = sample_table();
        // Reverse: [score, name, id]
        t.reorder_columns(&[2, 1, 0]);
        assert_eq!(t.columns[0].name, "score");
        assert_eq!(t.columns[1].name, "name");
        assert_eq!(t.columns[2].name, "id");
        assert_eq!(t.get(0, 0), Some(&CellValue::Float(9.5)));
        assert_eq!(t.get(0, 2), Some(&CellValue::Int(1)));
    }

    #[test]
    fn test_reorder_columns_remaps_edits() {
        let mut t = sample_table();
        t.set(0, 0, CellValue::Int(100)); // edit on col 0 (id)
        t.reorder_columns(&[2, 1, 0]); // id moves to position 2
        assert_eq!(t.get(0, 2), Some(&CellValue::Int(100)));
    }

    #[test]
    fn test_reorder_columns_wrong_length_noop() {
        let mut t = sample_table();
        t.reorder_columns(&[0, 1]); // wrong length
        assert!(!t.structural_changes);
        assert_eq!(t.columns[0].name, "id");
    }

    // --- Sorting ---

    #[test]
    fn test_sort_ascending() {
        let mut t = sample_table();
        t.sort_rows_by_column(1, true); // sort by name ascending
        assert_eq!(t.get(0, 1), Some(&CellValue::String("Alice".into())));
        assert_eq!(t.get(1, 1), Some(&CellValue::String("Bob".into())));
        assert_eq!(t.get(2, 1), Some(&CellValue::String("Charlie".into())));
    }

    #[test]
    fn test_sort_descending() {
        let mut t = sample_table();
        t.sort_rows_by_column(1, false); // sort by name descending
        assert_eq!(t.get(0, 1), Some(&CellValue::String("Charlie".into())));
        assert_eq!(t.get(1, 1), Some(&CellValue::String("Bob".into())));
        assert_eq!(t.get(2, 1), Some(&CellValue::String("Alice".into())));
    }

    #[test]
    fn test_sort_applies_edits_first() {
        let mut t = sample_table();
        t.set(0, 1, CellValue::String("Zara".into()));
        t.sort_rows_by_column(1, true);
        // Edits should be applied, so Zara sorts last
        assert!(t.edits.is_empty());
        assert_eq!(t.get(2, 1), Some(&CellValue::String("Zara".into())));
    }

    #[test]
    fn test_sort_with_nulls() {
        let mut t = sample_table();
        t.rows[1][1] = CellValue::Null;
        t.sort_rows_by_column(1, true);
        // Null should sort first
        assert_eq!(t.get(0, 1), Some(&CellValue::Null));
    }

    #[test]
    fn test_sort_numeric() {
        let mut t = sample_table();
        t.sort_rows_by_column(2, true); // sort by score
        assert_eq!(t.get(0, 2), Some(&CellValue::Float(7.0)));
        assert_eq!(t.get(1, 2), Some(&CellValue::Float(8.2)));
        assert_eq!(t.get(2, 2), Some(&CellValue::Float(9.5)));
    }

    #[test]
    fn test_sort_invalid_column_noop() {
        let mut t = sample_table();
        t.sort_rows_by_column(99, true);
        assert!(!t.structural_changes);
    }

    // --- is_modified ---

    #[test]
    fn test_is_modified_with_edits() {
        let mut t = sample_table();
        assert!(!t.is_modified());
        t.set(0, 0, CellValue::Int(100));
        assert!(t.is_modified());
    }

    #[test]
    fn test_is_modified_with_structural_changes() {
        let mut t = sample_table();
        t.insert_row(0);
        assert!(t.is_modified());
    }

    #[test]
    fn test_clear_modified() {
        let mut t = sample_table();
        t.insert_row(0);
        t.apply_edits();
        t.clear_modified();
        assert!(!t.is_modified());
    }

    // --- cmp_cell_values ---

    #[test]
    fn test_cmp_null_ordering() {
        assert_eq!(cmp_cell_values(&CellValue::Null, &CellValue::Null), std::cmp::Ordering::Equal);
        assert_eq!(cmp_cell_values(&CellValue::Null, &CellValue::Int(1)), std::cmp::Ordering::Less);
        assert_eq!(cmp_cell_values(&CellValue::Int(1), &CellValue::Null), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_cmp_int_float_cross() {
        assert_eq!(
            cmp_cell_values(&CellValue::Int(3), &CellValue::Float(3.5)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            cmp_cell_values(&CellValue::Float(2.5), &CellValue::Int(3)),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_cmp_strings_case_insensitive() {
        assert_eq!(
            cmp_cell_values(
                &CellValue::String("apple".into()),
                &CellValue::String("Banana".into())
            ),
            std::cmp::Ordering::Less
        );
    }

    // --- can_convert_value / can_convert_column ---

    #[test]
    fn test_null_converts_to_anything() {
        for t in &["String", "Int64", "Float64", "Boolean", "Date32", "Timestamp(Microsecond, None)"] {
            assert!(can_convert_value(&CellValue::Null, t));
        }
    }

    #[test]
    fn test_int_converts_to_string_float_bool() {
        assert!(can_convert_value(&CellValue::Int(42), "String"));
        assert!(can_convert_value(&CellValue::Int(42), "Float64"));
        assert!(can_convert_value(&CellValue::Int(1), "Boolean"));
    }

    #[test]
    fn test_int_does_not_convert_to_date() {
        assert!(!can_convert_value(&CellValue::Int(42), "Date32"));
    }

    #[test]
    fn test_string_to_int_valid() {
        assert!(can_convert_value(&CellValue::String("42".into()), "Int64"));
    }

    #[test]
    fn test_string_to_int_invalid() {
        assert!(!can_convert_value(&CellValue::String("hello".into()), "Int64"));
    }

    #[test]
    fn test_string_to_float_valid() {
        assert!(can_convert_value(&CellValue::String("3.14".into()), "Float64"));
    }

    #[test]
    fn test_string_to_float_invalid() {
        assert!(!can_convert_value(&CellValue::String("abc".into()), "Float64"));
    }

    #[test]
    fn test_string_to_bool_valid() {
        assert!(can_convert_value(&CellValue::String("true".into()), "Boolean"));
        assert!(can_convert_value(&CellValue::String("false".into()), "Boolean"));
        assert!(can_convert_value(&CellValue::String("yes".into()), "Boolean"));
        assert!(can_convert_value(&CellValue::String("0".into()), "Boolean"));
    }

    #[test]
    fn test_string_to_bool_invalid() {
        assert!(!can_convert_value(&CellValue::String("maybe".into()), "Boolean"));
    }

    #[test]
    fn test_string_to_date_valid() {
        assert!(can_convert_value(&CellValue::String("2024-01-15".into()), "Date32"));
    }

    #[test]
    fn test_string_to_date_invalid() {
        assert!(!can_convert_value(&CellValue::String("not-a-date".into()), "Date32"));
    }

    #[test]
    fn test_float_to_int_whole() {
        assert!(can_convert_value(&CellValue::Float(3.0), "Int64"));
    }

    #[test]
    fn test_float_to_int_fractional() {
        assert!(!can_convert_value(&CellValue::Float(3.14), "Int64"));
    }

    #[test]
    fn test_can_convert_column_mixed() {
        let mut t = sample_table();
        // Column 1 is "name" (strings: Alice, Bob, Charlie) - cannot convert to Int64
        assert!(!t.can_convert_column(1, "Int64"));
        // Column 0 is "id" (ints: 1, 2, 3) - can convert to String
        assert!(t.can_convert_column(0, "String"));
        // Column 0 can convert to Float64
        assert!(t.can_convert_column(0, "Float64"));
        // Column 2 is "score" (floats: 9.5, 7.0, 8.2) - 7.0 ok but 9.5/8.2 can't be Int
        assert!(!t.can_convert_column(2, "Int64"));
    }

    #[test]
    fn test_empty_string_converts_to_anything() {
        assert!(can_convert_value(&CellValue::String("".into()), "Int64"));
        assert!(can_convert_value(&CellValue::String("".into()), "Boolean"));
        assert!(can_convert_value(&CellValue::String("".into()), "Date32"));
    }
}
