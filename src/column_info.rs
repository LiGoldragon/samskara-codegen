use crate::datavalue;
use crate::error::CodegenError;

/// One column from CozoDB's `::columns` introspection result.
pub struct ColumnInfo {
    pub name: String,
    pub is_key: bool,
    pub index: u32,
    pub col_type: String,
}

/// Parse the JSON result of `::columns <relation>` into a vec of ColumnInfo.
///
/// CozoDB `::columns` returns headers: `["column", "is_key", "index", "type", ...]`
/// Values are wrapped in DataValue tags: `{"Str": "..."}`, `{"Bool": ...}`, etc.
pub fn from_columns_result(json: &serde_json::Value) -> Result<Vec<ColumnInfo>, CodegenError> {
    let rows = json
        .get("rows")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CodegenError::Schema("::columns result missing 'rows' array".into()))?;

    let mut columns = Vec::with_capacity(rows.len());
    for (idx, row) in rows.iter().enumerate() {
        let arr = row
            .as_array()
            .ok_or_else(|| CodegenError::Schema(format!("row {idx} is not an array")))?;

        let name = arr
            .first()
            .and_then(datavalue::as_str)
            .ok_or_else(|| CodegenError::Schema(format!("row {idx} missing column name")))?
            .to_string();

        let is_key = arr
            .get(1)
            .and_then(datavalue::as_bool)
            .ok_or_else(|| CodegenError::Schema(format!("row {idx} missing is_key")))?;

        // index is at position 2, type is at position 3
        let index = arr
            .get(2)
            .and_then(datavalue::as_i64)
            .ok_or_else(|| CodegenError::Schema(format!("row {idx} missing index")))?
            as u32;

        let col_type = arr
            .get(3)
            .and_then(datavalue::as_str)
            .ok_or_else(|| CodegenError::Schema(format!("row {idx} missing type")))?
            .to_string();

        columns.push(ColumnInfo {
            name,
            is_key,
            index,
            col_type,
        });
    }

    Ok(columns)
}
