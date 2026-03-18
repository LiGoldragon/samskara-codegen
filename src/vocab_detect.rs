use crate::column_info::ColumnInfo;
use crate::datavalue;
use crate::error::CodegenError;
use crate::schema_gen::EnumSchema;

/// Check if a relation qualifies as a vocab enum:
/// 1. Name ends with `_vocab`
/// 2. Exactly one key column
/// 3. Key column type is `String`
pub fn is_vocab_relation(name: &str, columns: &[ColumnInfo]) -> bool {
    if !name.ends_with("_vocab") {
        return false;
    }
    let key_cols: Vec<&ColumnInfo> = columns.iter().filter(|c| c.is_key).collect();
    key_cols.len() == 1 && key_cols[0].col_type == "String"
}

/// Query the vocab relation's rows and build an EnumSchema.
///
/// Enum name: strip `_vocab` suffix, convert to PascalCase.
/// Variants: sorted alphabetically by key value.
pub fn build_enum_schema(
    db: &criome_cozo::CriomeDb,
    relation_name: &str,
    columns: &[ColumnInfo],
) -> Result<EnumSchema, CodegenError> {
    let name = vocab_to_enum_name(relation_name);

    // Query all rows — the key column contains the enum variant names.
    // Use named binding syntax {name} for projection.
    let key_col = &columns.iter().find(|c| c.is_key).unwrap().name;
    let result = db.run_script(&format!(
        "?[val] := *{relation_name}{{{key_col}: val}}"
    ))?;
    let rows = result
        .get("rows")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CodegenError::Schema("vocab query missing 'rows'".into()))?;

    let mut variants: Vec<String> = rows
        .iter()
        .filter_map(|row| {
            datavalue::as_str(row.as_array()?.first()?).map(String::from)
        })
        .collect();
    variants.sort();

    Ok(EnumSchema { name, variants })
}

/// Convert `liveness_vocab` → `Liveness`, `commit_type_vocab` → `CommitType`.
fn vocab_to_enum_name(relation_name: &str) -> String {
    let base = relation_name.strip_suffix("_vocab").unwrap_or(relation_name);
    to_pascal_case(base)
}

/// Convert snake_case to PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocab_to_enum_name() {
        assert_eq!(vocab_to_enum_name("liveness_vocab"), "Liveness");
        assert_eq!(vocab_to_enum_name("commit_type_vocab"), "CommitType");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("agent_session"), "AgentSession");
        assert_eq!(to_pascal_case("thought"), "Thought");
        assert_eq!(to_pascal_case("world_commit_ref"), "WorldCommitRef");
    }
}
