use std::str::FromStr;

use crate::error::Error;

/// Cap'n Proto type corresponding to a CozoDB column type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapnpType {
    Text,
    Int64,
    Float64,
    Bool,
    Data,
}

impl CapnpType {
    /// Convert to the Cap'n Proto schema text representation.
    pub fn to_capnp_text(self) -> &'static str {
        match self {
            CapnpType::Text => "Text",
            CapnpType::Int64 => "Int64",
            CapnpType::Float64 => "Float64",
            CapnpType::Bool => "Bool",
            CapnpType::Data => "Data",
        }
    }
}

impl FromStr for CapnpType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "String" => Ok(CapnpType::Text),
            "Int" => Ok(CapnpType::Int64),
            "Float" => Ok(CapnpType::Float64),
            "Bool" => Ok(CapnpType::Bool),
            "Bytes" => Ok(CapnpType::Data),
            "Json" => Ok(CapnpType::Text),
            "List" => Ok(CapnpType::Data),
            other => Err(Error::TypeMap {
                detail: format!("unknown CozoDB type: {other}"),
            }),
        }
    }
}
