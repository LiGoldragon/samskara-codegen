use std::fmt;

/// Errors produced by codegen operations.
#[derive(Debug)]
pub enum Error {
    /// A CozoDB query failed during schema introspection.
    Query { detail: String },
    /// Schema introspection returned unexpected data.
    Schema { detail: String },
    /// Type mapping failed for a CozoDB column type.
    TypeMap { detail: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Query { detail } => write!(f, "query error: {detail}"),
            Error::Schema { detail } => {
                write!(f, "schema error: {detail}")
            }
            Error::TypeMap { detail } => {
                write!(f, "type map error: {detail}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<criome_cozo::Error> for Error {
    fn from(err: criome_cozo::Error) -> Self {
        Error::Query {
            detail: err.to_string(),
        }
    }
}
