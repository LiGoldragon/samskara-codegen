use crate::column_info::ColumnInfo;
use crate::datavalue;
use crate::error::Error;
use crate::schema_gen::EnumSchema;

/// Check if a relation name follows the PascalCase convention.
/// This is the fast visual signal — the Enum registry is the authority.
pub fn is_pascal_case(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Query the enum relation's rows and build an EnumSchema.
///
/// Finds the first key column and uses it to extract variant names.
/// Variants are sorted alphabetically.
pub fn build_enum_schema(
    db: &criome_cozo::CriomeDb,
    relation_name: &str,
    columns: &[ColumnInfo],
) -> Result<EnumSchema, Error> {
    let name = relation_name.to_string();

    let key_col = columns
        .iter()
        .find(|c| c.is_key)
        .ok_or_else(|| Error::Schema { detail: format!("{relation_name} has no key column") })?;

    let result = db.run_script(&format!(
        "?[val] := *{relation_name}{{{}: val}}", key_col.name
    ))?;
    let rows = result
        .get("rows")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::Schema { detail: "enum query missing 'rows'".into() })?;

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
    fn test_is_pascal_case() {
        assert!(is_pascal_case("Phase"));
        assert!(is_pascal_case("Dignity"));
        assert!(is_pascal_case("Domain"));
        assert!(!is_pascal_case("thought"));
        assert!(!is_pascal_case("agent_session"));
        assert!(!is_pascal_case("samskrta"));
    }
}
