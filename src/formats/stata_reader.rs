use crate::data::{CellValue, ColumnInfo, DataTable};
use crate::formats::FormatReader;
use anyhow::{Context, Result};
use dta::stata::dta::dta_reader::DtaReader;
use dta::stata::dta::long_string_ref::LongStringRef;
use dta::stata::dta::long_string_table::LongStringTable;
use dta::stata::dta::value::Value as DtaValue;
use dta::stata::dta::variable_type::VariableType;
use std::path::Path;

pub struct StataReader;

impl FormatReader for StataReader {
    fn name(&self) -> &str {
        "Stata"
    }

    fn extensions(&self) -> &[&str] {
        &["dta"]
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        let header_reader = DtaReader::new()
            .from_path(path)
            .with_context(|| format!("opening Stata file {}", path.display()))?;
        let schema_reader = header_reader.read_header()?;
        let mut characteristic_reader = schema_reader.read_schema()?;
        characteristic_reader.skip_to_end()?;

        let columns: Vec<ColumnInfo> = characteristic_reader
            .schema()
            .variables()
            .iter()
            .map(|v| ColumnInfo {
                name: v.name().to_string(),
                data_type: variable_type_string(v.variable_type()).to_string(),
            })
            .collect();

        let mut record_reader = characteristic_reader.into_record_reader()?;

        // First pass: collect values, but for `LongStringRef` we keep the
        // ref and resolve it after we walk the strL section below.
        #[derive(Clone)]
        enum Pending {
            Resolved(CellValue),
            LongRef(LongStringRef),
        }

        let mut pending_rows: Vec<Vec<Pending>> = Vec::new();
        while let Some(record) = record_reader.read_record()? {
            let mut row: Vec<Pending> = Vec::with_capacity(record.values().len());
            for value in record.values() {
                row.push(match value {
                    DtaValue::LongStringRef(r) => Pending::LongRef(*r),
                    other => Pending::Resolved(dta_value_to_cell(other)),
                });
            }
            pending_rows.push(row);
        }

        // strL pass: only present for v117+. read_remaining_into is a no-op
        // on older releases.
        let mut long_string_reader = record_reader.into_long_string_reader()?;
        let encoding = long_string_reader.encoding();
        let mut long_strings = LongStringTable::for_reading();
        long_string_reader.read_remaining_into(&mut long_strings)?;

        let rows: Vec<Vec<CellValue>> = pending_rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| match cell {
                        Pending::Resolved(v) => v,
                        Pending::LongRef(key) => match long_strings.get(&key) {
                            Some(ls) => match ls.data_str(encoding) {
                                Some(s) => CellValue::String(s.into_owned()),
                                None => CellValue::Binary(ls.data().to_vec()),
                            },
                            None => CellValue::Null,
                        },
                    })
                    .collect()
            })
            .collect();

        Ok(DataTable {
            columns,
            rows,
            edits: std::collections::HashMap::new(),
            source_path: Some(path.to_string_lossy().to_string()),
            format_name: Some("Stata".to_string()),
            structural_changes: false,
            total_rows: None,
            row_offset: 0,
            marks: std::collections::HashMap::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            db_meta: None,
        })
    }
}

fn variable_type_string(t: VariableType) -> &'static str {
    match t {
        VariableType::Byte => "Int8",
        VariableType::Int => "Int16",
        VariableType::Long => "Int32",
        VariableType::Float => "Float32",
        VariableType::Double => "Float64",
        VariableType::FixedString(_) | VariableType::LongString => "Utf8",
    }
}

fn dta_value_to_cell(value: &DtaValue<'_>) -> CellValue {
    match value {
        DtaValue::Byte(v) => v
            .present()
            .map(|n| CellValue::Int(i64::from(n)))
            .unwrap_or(CellValue::Null),
        DtaValue::Int(v) => v
            .present()
            .map(|n| CellValue::Int(i64::from(n)))
            .unwrap_or(CellValue::Null),
        DtaValue::Long(v) => v
            .present()
            .map(|n| CellValue::Int(i64::from(n)))
            .unwrap_or(CellValue::Null),
        DtaValue::Float(v) => v
            .present()
            .map(|n| CellValue::Float(f64::from(n)))
            .unwrap_or(CellValue::Null),
        DtaValue::Double(v) => v.present().map(CellValue::Float).unwrap_or(CellValue::Null),
        DtaValue::String(s) => CellValue::String(s.as_ref().to_string()),
        // Resolved by the caller — should never reach here.
        DtaValue::LongStringRef(_) => CellValue::Null,
    }
}
