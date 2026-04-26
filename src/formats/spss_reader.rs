use crate::data::{CellValue, ColumnInfo, DataTable};
use crate::formats::FormatReader;
use anyhow::{Context, Result};
use std::path::Path;

// `ambers` returns data as `arrow` v57 RecordBatch. We import that arrow
// version under an alias to coexist with the project's main arrow v54.
use arrow57::array::{
    Array, BinaryViewArray, Date32Array, DurationMicrosecondArray, Float64Array, StringArray,
    StringViewArray, TimestampMicrosecondArray,
};
use arrow57::datatypes::{DataType as Arrow57DataType, TimeUnit as Arrow57TimeUnit};

pub struct SpssReader;

impl FormatReader for SpssReader {
    fn name(&self) -> &str {
        "SPSS"
    }

    fn extensions(&self) -> &[&str] {
        &["sav", "zsav"]
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        let (batch, _meta) = ambers::read_sav(path)
            .with_context(|| format!("opening SPSS file {}", path.display()))?;

        let schema = batch.schema();
        let columns: Vec<ColumnInfo> = schema
            .fields()
            .iter()
            .map(|f| ColumnInfo {
                name: f.name().clone(),
                data_type: arrow57_type_string(f.data_type()).to_string(),
            })
            .collect();

        let num_rows = batch.num_rows();
        let num_cols = batch.num_columns();
        let mut rows: Vec<Vec<CellValue>> = Vec::with_capacity(num_rows);
        for row_idx in 0..num_rows {
            let mut row = Vec::with_capacity(num_cols);
            for col_idx in 0..num_cols {
                let array = batch.column(col_idx);
                row.push(arrow57_value_to_cell(array.as_ref(), row_idx));
            }
            rows.push(row);
        }

        Ok(DataTable {
            columns,
            rows,
            edits: std::collections::HashMap::new(),
            source_path: Some(path.to_string_lossy().to_string()),
            format_name: Some("SPSS".to_string()),
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

fn arrow57_type_string(dt: &Arrow57DataType) -> &'static str {
    match dt {
        Arrow57DataType::Boolean => "Boolean",
        Arrow57DataType::Int8 => "Int8",
        Arrow57DataType::Int16 => "Int16",
        Arrow57DataType::Int32 => "Int32",
        Arrow57DataType::Int64 => "Int64",
        Arrow57DataType::Float32 => "Float32",
        Arrow57DataType::Float64 => "Float64",
        Arrow57DataType::Utf8 | Arrow57DataType::Utf8View | Arrow57DataType::LargeUtf8 => "Utf8",
        Arrow57DataType::Date32 | Arrow57DataType::Date64 => "Date",
        Arrow57DataType::Timestamp(_, _) => "DateTime",
        Arrow57DataType::Duration(_) => "Utf8",
        _ => "Utf8",
    }
}

fn arrow57_value_to_cell(array: &dyn Array, row: usize) -> CellValue {
    if array.is_null(row) {
        return CellValue::Null;
    }
    match array.data_type() {
        Arrow57DataType::Float64 => {
            let a = array.as_any().downcast_ref::<Float64Array>().unwrap();
            CellValue::Float(a.value(row))
        }
        Arrow57DataType::Utf8 => {
            let a = array.as_any().downcast_ref::<StringArray>().unwrap();
            CellValue::String(a.value(row).trim_end().to_string())
        }
        Arrow57DataType::Utf8View => {
            let a = array.as_any().downcast_ref::<StringViewArray>().unwrap();
            CellValue::String(a.value(row).trim_end().to_string())
        }
        Arrow57DataType::BinaryView => {
            let a = array.as_any().downcast_ref::<BinaryViewArray>().unwrap();
            CellValue::Binary(a.value(row).to_vec())
        }
        Arrow57DataType::Date32 => {
            let a = array.as_any().downcast_ref::<Date32Array>().unwrap();
            let days = a.value(row);
            chrono::NaiveDate::from_num_days_from_ce_opt(days + 719_163)
                .map(|d| CellValue::Date(d.format("%Y-%m-%d").to_string()))
                .unwrap_or(CellValue::Null)
        }
        Arrow57DataType::Timestamp(unit, _) => {
            let us = match unit {
                Arrow57TimeUnit::Microsecond => array
                    .as_any()
                    .downcast_ref::<TimestampMicrosecondArray>()
                    .unwrap()
                    .value(row),
                _ => return CellValue::String(format!("ts({:?})", unit)),
            };
            let secs = us.div_euclid(1_000_000);
            let nsecs = (us.rem_euclid(1_000_000) * 1_000) as u32;
            chrono::DateTime::from_timestamp(secs, nsecs)
                .map(|dt| CellValue::DateTime(dt.format("%Y-%m-%d %H:%M:%S%.6f").to_string()))
                .unwrap_or(CellValue::Null)
        }
        Arrow57DataType::Duration(unit) => match unit {
            Arrow57TimeUnit::Microsecond => {
                let a = array
                    .as_any()
                    .downcast_ref::<DurationMicrosecondArray>()
                    .unwrap();
                let us = a.value(row);
                let total_secs = us.div_euclid(1_000_000);
                let frac_us = us.rem_euclid(1_000_000);
                let h = total_secs / 3600;
                let m = (total_secs % 3600) / 60;
                let s = total_secs % 60;
                if frac_us == 0 {
                    CellValue::String(format!("{h:02}:{m:02}:{s:02}"))
                } else {
                    CellValue::String(format!("{h:02}:{m:02}:{s:02}.{:06}", frac_us))
                }
            }
            _ => CellValue::String(format!("dur({:?})", unit)),
        },
        _ => CellValue::String(format!("{:?}", array.data_type())),
    }
}
