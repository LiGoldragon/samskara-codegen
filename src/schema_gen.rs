use crate::column_info::{self, ColumnInfo};
use crate::datavalue;
use crate::error::CodegenError;
use crate::type_map::CapnpType;
use crate::vocab_detect;

/// A full relation schema ready for Cap'n Proto struct generation.
pub struct RelationSchema {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

/// An enum derived from a `*_vocab` relation.
pub struct EnumSchema {
    pub name: String,
    pub variants: Vec<String>,
}

/// The core codegen object: holds all relation schemas and enums discovered
/// from a CozoDB instance, and generates deterministic `.capnp` output.
pub struct SchemaGenerator {
    pub relations: Vec<RelationSchema>,
    pub enums: Vec<EnumSchema>,
}

impl SchemaGenerator {
    /// Introspect a CozoDB instance: query `::relations` and `::columns` for
    /// each relation, detect vocab enums, and build the full schema.
    pub fn from_db(db: &criome_cozo::CriomeDb) -> Result<Self, CodegenError> {
        let relations_result = db.run_script("::relations")?;
        let rows = relations_result
            .get("rows")
            .and_then(|v| v.as_array())
            .ok_or_else(|| CodegenError::Schema("::relations missing 'rows'".into()))?;

        // Extract relation names (first column of each row, DataValue-wrapped)
        let mut relation_names: Vec<String> = rows
            .iter()
            .filter_map(|row| {
                datavalue::as_str(row.as_array()?.first()?).map(String::from)
            })
            .collect();
        relation_names.sort();

        let mut relations = Vec::new();
        let mut enums = Vec::new();

        for name in &relation_names {
            let columns_result = db.run_script(&format!("::columns {name}"))?;
            let columns = column_info::from_columns_result(&columns_result)?;

            if vocab_detect::is_vocab_relation(name, &columns) {
                let enum_schema = vocab_detect::build_enum_schema(db, name, &columns)?;
                enums.push(enum_schema);
            } else {
                relations.push(RelationSchema {
                    name: name.clone(),
                    columns,
                });
            }
        }

        Ok(Self { relations, enums })
    }

    /// Generate deterministic `.capnp` schema text.
    pub fn to_capnp_text(&self) -> Result<String, CodegenError> {
        let mut out = String::new();

        // File ID: blake3 of sorted relation names, truncated to u64
        let file_id = self.file_id();
        out.push_str(&format!("@0x{file_id:016x};\n\n"));

        // Enums first (sorted alphabetically by name)
        let mut sorted_enums: Vec<&EnumSchema> = self.enums.iter().collect();
        sorted_enums.sort_by(|a, b| a.name.cmp(&b.name));
        for e in &sorted_enums {
            out.push_str(&format!("enum {} {{\n", e.name));
            for (i, variant) in e.variants.iter().enumerate() {
                out.push_str(&format!("  {} @{};\n", to_camel_case(variant), i));
            }
            out.push_str("}\n\n");
        }

        // Structs (sorted alphabetically by relation name)
        for rel in &self.relations {
            let struct_name = to_pascal_case(&rel.name);
            out.push_str(&format!("struct {} {{\n", struct_name));
            for col in &rel.columns {
                let field_name = to_camel_case(&col.name);
                let capnp_type: CapnpType = col.col_type.parse()?;
                out.push_str(&format!(
                    "  {} @{} :{};",
                    field_name, col.index, capnp_type.to_capnp_text()
                ));
                if col.is_key {
                    out.push_str("  # key");
                }
                out.push('\n');
            }
            out.push_str("}\n\n");
        }

        Ok(out)
    }

    /// Compute the blake3 hash of the generated schema for content addressing.
    pub fn schema_hash(&self) -> Result<blake3::Hash, CodegenError> {
        let text = self.to_capnp_text()?;
        Ok(blake3::hash(text.as_bytes()))
    }

    /// Compute the file ID: blake3 of sorted relation names, truncated to u64.
    fn file_id(&self) -> u64 {
        let mut hasher = blake3::Hasher::new();
        for rel in &self.relations {
            hasher.update(rel.name.as_bytes());
        }
        for e in &self.enums {
            hasher.update(e.name.as_bytes());
        }
        let hash = hasher.finalize();
        let bytes: [u8; 8] = hash.as_bytes()[..8].try_into().unwrap();
        // Cap'n Proto requires the high bit set in file IDs
        u64::from_le_bytes(bytes) | 0x8000000000000000
    }
}

/// Convert snake_case to PascalCase: `agent_session` → `AgentSession`.
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

/// Convert snake_case to camelCase: `created_ts` → `createdTs`.
fn to_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split('_').collect();
    let mut result = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            result.push_str(part);
        } else {
            let mut chars = part.chars();
            if let Some(c) = chars.next() {
                let upper: String = c.to_uppercase().collect();
                result.push_str(&upper);
                result.push_str(chars.as_str());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("thought"), "Thought");
        assert_eq!(to_pascal_case("agent_session"), "AgentSession");
        assert_eq!(to_pascal_case("world_commit_ref"), "WorldCommitRef");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("created_ts"), "createdTs");
        assert_eq!(to_camel_case("name"), "name");
        assert_eq!(to_camel_case("ref_type"), "refType");
    }
}
