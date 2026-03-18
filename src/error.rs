use std::fmt;

/// Errors produced by samskara-codegen operations.
#[derive(Debug)]
pub enum CodegenError {
    /// A CozoDB query failed during schema introspection.
    Query(String),
    /// Schema introspection returned unexpected data.
    Schema(String),
    /// Type mapping failed for a CozoDB column type.
    TypeMap(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::Query(msg) => write!(f, "codegen query error: {msg}"),
            CodegenError::Schema(msg) => write!(f, "codegen schema error: {msg}"),
            CodegenError::TypeMap(msg) => write!(f, "codegen type map error: {msg}"),
        }
    }
}

impl std::error::Error for CodegenError {}

impl From<criome_cozo::CozoError> for CodegenError {
    fn from(err: criome_cozo::CozoError) -> Self {
        CodegenError::Query(err.to_string())
    }
}
