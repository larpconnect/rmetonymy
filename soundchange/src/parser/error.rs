use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum SoundChangeParseError {
    #[error("Pest parse error: {0}")]
    PestError(String),
    #[error("AST conversion error: {0}")]
    ConversionError(String),
    #[error("Preamble reference error: {0}")]
    ReferenceError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}
