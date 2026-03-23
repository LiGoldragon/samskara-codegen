use crate::column_info::{self, ColumnInfo};
use crate::datavalue;
use crate::error::Error;
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
    pub fn from_db(db: &criome_cozo::CriomeDb) -> Result<Self, Error> {
        // Query the Domain registry — the authority for which relations are domains.
        // PascalCase is the convention (fast visual signal).
        // The registry is the truth (authoritative data signal).
        let enum_registry: std::collections::HashSet<String> = db
            .run_script("?[name] := *Domain{name}")
            .ok()
            .and_then(|v| v.get("rows")?.as_array().cloned())
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| {
                        datavalue::as_str(row.as_array()?.first()?).map(String::from)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let has_registry = !enum_registry.is_empty();

        let relations_result = db.run_script("::relations")?;
        let rows = relations_result
            .get("rows")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::Schema { detail: "::relations missing 'rows'".into() })?;

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

            let in_registry = enum_registry.contains(name);
            let is_pascal = vocab_detect::is_pascal_case(name);

            // Registry is authoritative when it exists.
            // PascalCase alone is the fallback when no registry.
            let in_domain = if has_registry { in_registry } else { is_pascal };

            // Single-key domains → capnp enum (variants are the key values).
            // Composite-key domains → capnp struct (like any other relation).
            let key_count = columns.iter().filter(|c| c.is_key).count();
            let is_enum = in_domain && key_count == 1;

            if is_enum {
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
    pub fn to_capnp_text(&self) -> Result<String, Error> {
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
                out.push_str(&format!("  {} @{};\n", to_capnp_enumerant(variant), i));
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
    pub fn schema_hash(&self) -> Result<blake3::Hash, Error> {
        let text = self.to_capnp_text()?;
        Ok(blake3::hash(text.as_bytes()))
    }

    /// Generate deterministic CozoScript `:create` statements for all relations.
    /// Requires DB access to get full column info for enum relations.
    /// This is the DB→.cozo projection: a readable artifact, not an authoritative source.
    pub fn to_cozo_init_text(&self, db: &criome_cozo::CriomeDb) -> Result<String, Error> {
        let mut out = String::new();
        let hash = self.schema_hash()?;
        out.push_str(&format!("# Generated from live DB — do not edit manually\n"));
        out.push_str(&format!("# Schema hash: {hash}\n\n"));

        // Collect all relation names (enums + non-enums), sorted
        let mut all_names: Vec<&str> = self.enums.iter().map(|e| e.name.as_str())
            .chain(self.relations.iter().map(|r| r.name.as_str()))
            .collect();
        all_names.sort();

        for name in all_names {
            let columns_result = db.run_script(&format!("::columns {name}"))
                .map_err(|e| Error::Query { detail: e.to_string() })?;
            let columns = column_info::from_columns_result(&columns_result)?;

            out.push_str(&format!(":create {name} {{\n"));
            let key_count = columns.iter().filter(|c| c.is_key).count();
            for (i, col) in columns.iter().enumerate() {
                let separator = if col.is_key && i == key_count - 1 && key_count < columns.len() {
                    " =>"
                } else if i < columns.len() - 1 {
                    ","
                } else {
                    ""
                };
                out.push_str(&format!("  {}: {}{}\n", col.name, col.col_type, separator));
            }
            out.push_str("}\n\n");
        }

        Ok(out)
    }

    /// Generate deterministic CozoScript `:put` statements for all seed data.
    /// Queries all manifest-phase rows from versioned relations and emits them
    /// as reproducible seed data. Rows sorted by key for determinism.
    pub fn to_cozo_seed_text(&self, db: &criome_cozo::CriomeDb) -> Result<String, Error> {
        let mut out = String::new();
        let hash = self.schema_hash()?;
        out.push_str(&format!("# Generated from live DB — do not edit manually\n"));
        out.push_str(&format!("# Schema hash: {hash}\n\n"));

        // Collect all relation names (enums + non-enums), sorted
        let mut all_names: Vec<&str> = self.enums.iter().map(|e| e.name.as_str())
            .chain(self.relations.iter().map(|r| r.name.as_str()))
            .collect();
        all_names.sort();

        for name in all_names {
            let columns_result = db.run_script(&format!("::columns {name}"))
                .map_err(|e| Error::Query { detail: e.to_string() })?;
            let columns = column_info::from_columns_result(&columns_result)?;

            if columns.is_empty() {
                continue;
            }

            let col_names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
            let has_phase = col_names.contains(&"phase");

            // Query rows — filter to manifest phase if applicable
            let col_list = col_names.join(", ");
            let query = if has_phase {
                format!(r#"?[{col_list}] := *{name}{{{col_list}}}, phase == "manifest" :order {}"#,
                    col_names[0])
            } else {
                format!("?[{col_list}] := *{name}{{{col_list}}} :order {}", col_names[0])
            };

            let result = db.run_script(&query)
                .map_err(|e| Error::Query { detail: format!("seed query {name}: {e}") })?;

            let rows = result.get("rows").and_then(|v| v.as_array());
            let rows = match rows {
                Some(r) if !r.is_empty() => r,
                _ => continue,
            };

            // Build key => value clause
            let key_count = columns.iter().filter(|c| c.is_key).count();
            let kv_clause = if key_count < columns.len() {
                let keys = col_names[..key_count].join(", ");
                let vals = col_names[key_count..].join(", ");
                format!("{keys} => {vals}")
            } else {
                col_names.join(", ")
            };

            out.push_str(&format!("?[{col_list}] <- [\n"));
            for row in rows {
                if let Some(arr) = row.as_array() {
                    let vals: Vec<String> = arr.iter().map(|v| {
                        if let Some(s) = v.get("Str").and_then(|s| s.as_str()).or(v.as_str()) {
                            // CozoScript has no escape sequences except \" in double-quoted
                            // and \' in single-quoted strings. Backslashes are literal.
                            // Use double quotes; only escape embedded double quotes.
                            let escaped = s.replace('"', "\\\"");
                            format!("\"{escaped}\"")
                        } else if let Some(b) = v.get("Bool").and_then(|b| b.as_bool()).or(v.as_bool()) {
                            if b { "true".into() } else { "false".into() }
                        } else if let Some(n) = v.get("Num") {
                            if let Some(i) = n.get("Int").and_then(|i| i.as_i64()) {
                                i.to_string()
                            } else if let Some(f) = n.get("Float").and_then(|f| f.as_f64()) {
                                f.to_string()
                            } else {
                                "null".into()
                            }
                        } else if let Some(i) = v.as_i64() {
                            i.to_string()
                        } else if let Some(f) = v.as_f64() {
                            f.to_string()
                        } else {
                            "null".into()
                        }
                    }).collect();
                    out.push_str(&format!("  [{}],\n", vals.join(", ")));
                }
            }
            out.push_str(&format!("]\n:put {name} {{ {kv_clause} }}\n\n"));
        }

        Ok(out)
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

/// Convert delimited names to PascalCase: `agent_session` → `AgentSession`, `annas-archive` → `AnnasArchive`.
fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-')
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

/// Convert enum variant to capnp enumerant: lowercase first char.
/// Handles snake_case (`commit_type` → `commitType`), PascalCase (`CommitType` → `commitType`),
/// and plain lowercase (`sol` → `sol`).
fn to_capnp_enumerant(s: &str) -> String {
    // If it contains separators (underscores or hyphens), treat as delimited → camelCase
    if s.contains('_') || s.contains('-') {
        return to_camel_case(s);
    }
    // Otherwise, lowercase the first character
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => {
            let lower: String = c.to_lowercase().collect();
            lower + chars.as_str()
        }
    }
}

/// Convert delimited names to camelCase: `created_ts` → `createdTs`, `read-write` → `readWrite`.
/// Splits on both underscores and hyphens (Cap'n Proto identifiers cannot contain either).
fn to_camel_case(s: &str) -> String {
    let parts: Vec<&str> = s.split(|c| c == '_' || c == '-').collect();
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
        assert_eq!(to_pascal_case("annas-archive"), "AnnasArchive");
        assert_eq!(to_pascal_case("rust-analyzer"), "RustAnalyzer");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("created_ts"), "createdTs");
        assert_eq!(to_camel_case("name"), "name");
        assert_eq!(to_camel_case("ref_type"), "refType");
        assert_eq!(to_camel_case("read-write"), "readWrite");
        assert_eq!(to_camel_case("read-only"), "readOnly");
        assert_eq!(to_camel_case("rust-analyzer"), "rustAnalyzer");
    }

    #[test]
    fn test_to_capnp_enumerant() {
        assert_eq!(to_capnp_enumerant("allowed"), "allowed");
        assert_eq!(to_capnp_enumerant("read-write"), "readWrite");
        assert_eq!(to_capnp_enumerant("read-only"), "readOnly");
        assert_eq!(to_capnp_enumerant("commit_type"), "commitType");
        assert_eq!(to_capnp_enumerant("CommitType"), "commitType");
    }
}
