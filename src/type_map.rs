use crate::error::Error;

/// Cap'n Proto type resolved from a CozoDB column via the field_type graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapnpType {
    /// Reference to a domain enum (e.g. "Element", "Phase").
    DomainRef(String),
    /// Typed integer: struct { measure :UInt16; unit :UInt16; magnitude :Int64 }.
    TypedInt,
    /// Boolean leaf — Dignaga's pratyaksha.
    Bool,
    /// Content-addressed bytes — Saturn's archive.
    Data,
    /// Legacy: bare Text (only used when field_type graph is not loaded).
    LegacyText,
    /// Legacy: bare Int64 (only used when field_type graph is not loaded).
    LegacyInt64,
    /// Legacy: bare Float64 (only used when field_type graph is not loaded).
    LegacyFloat64,
}

impl CapnpType {
    /// Convert to the Cap'n Proto schema text representation.
    pub fn to_capnp_text(&self) -> String {
        match self {
            CapnpType::DomainRef(name) => name.clone(),
            CapnpType::TypedInt => "TypedInt".to_string(),
            CapnpType::Bool => "Bool".to_string(),
            CapnpType::Data => "Data".to_string(),
            CapnpType::LegacyText => "Text".to_string(),
            CapnpType::LegacyInt64 => "Int64".to_string(),
            CapnpType::LegacyFloat64 => "Float64".to_string(),
        }
    }

    /// Resolve a field type from the field_type graph entry.
    /// kind: "domain", "bool", "int", "data"
    /// target_domain: domain name when kind=domain
    pub fn from_field_type(kind: &str, target_domain: &str) -> Result<Self, Error> {
        match kind {
            "domain" => {
                if target_domain.is_empty() {
                    return Err(Error::TypeMap {
                        detail: "kind=domain but target_domain is empty".into(),
                    });
                }
                Ok(CapnpType::DomainRef(target_domain.to_string()))
            }
            "bool" => Ok(CapnpType::Bool),
            "int" => Ok(CapnpType::TypedInt),
            "data" => Ok(CapnpType::Data),
            other => Err(Error::TypeMap {
                detail: format!("unknown ScalarKind: {other}"),
            }),
        }
    }

    /// Fallback: resolve from CozoDB column type when no field_type entry exists.
    /// This is the legacy path — used only when field_type graph is not loaded.
    /// Preserves backward compatibility: String→Text, Int→Int64.
    pub fn from_cozo_type(col_type: &str) -> Result<Self, Error> {
        match col_type {
            "String" => Ok(CapnpType::LegacyText),
            "Int" => Ok(CapnpType::LegacyInt64),
            "Float" => Ok(CapnpType::LegacyFloat64),
            "Bool" => Ok(CapnpType::Bool),
            "Bytes" => Ok(CapnpType::Data),
            "Json" => Ok(CapnpType::LegacyText),
            "List" => Ok(CapnpType::Data),
            other => Err(Error::TypeMap {
                detail: format!("unknown CozoDB type: {other}"),
            }),
        }
    }
}
