use crate::data::{CellValue, ColumnInfo, DataTable};
use crate::formats::FormatReader;
use anyhow::{Result, anyhow};
use std::path::Path;

pub struct NetCdfReader;

impl FormatReader for NetCdfReader {
    fn name(&self) -> &str {
        "NetCDF"
    }

    fn extensions(&self) -> &[&str] {
        &["nc"]
    }

    fn read_file(&self, path: &Path) -> Result<DataTable> {
        read_netcdf3(path)
    }
}

fn read_netcdf3(path: &Path) -> Result<DataTable> {
    // `netcdf3::ReadError` doesn't implement `std::error::Error` (snafu-based
    // upstream), so anyhow's `Context` extension methods don't kick in. Format
    // the error manually instead.
    let mut reader = netcdf3::FileReader::open(path)
        .map_err(|e| anyhow!("opening NetCDF-3 file {}: {e}", path.display()))?;

    // Group all 1D variables by their dimension. Pick the dimension with the
    // largest set of variables as the "main table" - each variable becomes a
    // column whose row index is the dimension axis. Multi-dimensional
    // variables and 0D scalars are skipped (with a count surfaced via
    // `format_name`).
    let var_names = reader.data_set().get_var_names();

    let mut grouped: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    let mut multi_dim_skipped = 0usize;
    let mut scalar_skipped = 0usize;
    for var_name in &var_names {
        let var = match reader.data_set().get_var(var_name) {
            Some(v) => v,
            None => continue,
        };
        match var.num_dims() {
            0 => scalar_skipped += 1,
            1 => {
                let dim_name = var.dim_names().into_iter().next().unwrap_or_default();
                grouped.entry(dim_name).or_default().push(var_name.clone());
            }
            _ => multi_dim_skipped += 1,
        }
    }

    // Pick the dimension with the most variables (ties broken by name).
    let (chosen_dim, chosen_vars) = grouped
        .iter()
        .max_by_key(|(_, v)| v.len())
        .map(|(d, v)| (d.clone(), v.clone()))
        .ok_or_else(|| {
            anyhow!(
                "NetCDF file contains no 1D variables (skipped {scalar_skipped} scalars, \
                 {multi_dim_skipped} multi-dimensional variables)"
            )
        })?;

    let row_count = reader
        .data_set()
        .get_dim(&chosen_dim)
        .map(|d| d.size())
        .unwrap_or(0);

    let mut columns: Vec<ColumnInfo> = Vec::with_capacity(chosen_vars.len());
    let mut column_data: Vec<Vec<CellValue>> = Vec::with_capacity(chosen_vars.len());
    for var_name in &chosen_vars {
        let dv = reader
            .read_var(var_name)
            .map_err(|e| anyhow!("reading NetCDF variable `{var_name}`: {e}"))?;
        let (data_type, cells) = data_vector_to_cells(&dv);
        columns.push(ColumnInfo {
            name: var_name.clone(),
            data_type,
        });
        column_data.push(cells);
    }

    // Pivot column-major to row-major.
    let mut rows = Vec::with_capacity(row_count);
    for r in 0..row_count {
        let mut row = Vec::with_capacity(columns.len());
        for col in &column_data {
            row.push(col.get(r).cloned().unwrap_or(CellValue::Null));
        }
        rows.push(row);
    }

    let mut format_name = "NetCDF".to_string();
    let other_dims = grouped.len().saturating_sub(1);
    if multi_dim_skipped > 0 || scalar_skipped > 0 || other_dims > 0 {
        format_name.push_str(" (");
        let mut bits = Vec::new();
        if multi_dim_skipped > 0 {
            bits.push(format!("{multi_dim_skipped} multi-D vars skipped"));
        }
        if scalar_skipped > 0 {
            bits.push(format!("{scalar_skipped} scalars skipped"));
        }
        if other_dims > 0 {
            bits.push(format!("{other_dims} other dim(s) skipped"));
        }
        format_name.push_str(&bits.join(", "));
        format_name.push(')');
    }

    let mut table = DataTable::empty();
    table.columns = columns;
    table.rows = rows;
    table.source_path = Some(path.to_string_lossy().to_string());
    table.format_name = Some(format_name);
    Ok(table)
}

fn data_vector_to_cells(dv: &netcdf3::DataVector) -> (String, Vec<CellValue>) {
    use netcdf3::DataVector as DV;
    match dv {
        DV::I8(v) => (
            "Int32".to_string(),
            v.iter().map(|&x| CellValue::Int(x as i64)).collect(),
        ),
        DV::U8(v) => (
            "Int32".to_string(),
            v.iter().map(|&x| CellValue::Int(x as i64)).collect(),
        ),
        DV::I16(v) => (
            "Int32".to_string(),
            v.iter().map(|&x| CellValue::Int(x as i64)).collect(),
        ),
        DV::I32(v) => (
            "Int32".to_string(),
            v.iter().map(|&x| CellValue::Int(x as i64)).collect(),
        ),
        DV::F32(v) => (
            "Float32".to_string(),
            v.iter()
                .map(|&x| {
                    if x.is_nan() {
                        CellValue::Null
                    } else {
                        CellValue::Float(x as f64)
                    }
                })
                .collect(),
        ),
        DV::F64(v) => (
            "Float64".to_string(),
            v.iter()
                .map(|&x| {
                    if x.is_nan() {
                        CellValue::Null
                    } else {
                        CellValue::Float(x)
                    }
                })
                .collect(),
        ),
    }
}
