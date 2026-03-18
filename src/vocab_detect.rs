use crate::column_info::ColumnInfo;
use crate::datavalue;
use crate::error::CodegenError;
use crate::schema_gen::EnumSchema;

/// Check if a relation qualifies as an enum type:
/// 1. Name starts with an uppercase letter (PascalCase convention)
/// 2. Exactly one key column
/// 3. Key column type is `String`
pub fn is_vocab_relation(name: &str, columns: &[ColumnInfo]) -> bool {
    let starts_upper = name.chars().next().is_some_and(|c| c.is_uppercase());
    if !starts_upper {
        return false;
    }
    let key_cols: Vec<&ColumnInfo> = columns.iter().filter(|c| c.is_key).collect();
    key_cols.len() == 1 && key_cols[0].col_type == "String"
}

/// Query the enum relation's rows and build an EnumSchema.
///
/// Enum name: the relation name directly (already PascalCase).
/// Variants: sorted alphabetically by key value.
pub fn build_enum_schema(
    db: &criome_cozo::CriomeDb,
    relation_name: &str,
    columns: &[ColumnInfo],
) -> Result<EnumSchema, CodegenError> {
    // Relation name IS the enum name (already PascalCase)
    let name = relation_name.to_string();

    // Query all rows — the key column contains the enum variant names.
    let key_col = &columns.iter().find(|c| c.is_key).unwrap().name;
    let result = db.run_script(&format!(
        "?[val] := *{relation_name}{{{key_col}: val}}"
    ))?;
    let rows = result
        .get("rows")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CodegenError::Schema("enum query missing 'rows'".into()))?;

    let mut variants: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            datavalue::as_str(row.as_array()?.first()?).map(String::from)
        })
        .collect();
    variants.sort();

    Ok(EnumSchema { name, variants })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vocab_uppercase() {
        let cols = vec![ColumnInfo {
            name: "name".to_string(),
            is_key: true,
            index: 0,
            col_type: "String".to_string(),
        }];
        assert!(is_vocab_relation("Phase", &cols));
        assert!(is_vocab_relation("Dignity", &cols));
        assert!(!is_vocab_relation("thought", &cols));
        assert!(!is_vocab_relation("agent_session", &cols));
    }
}
