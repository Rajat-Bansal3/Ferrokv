#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("incomplete data")]
    Incomplete,
    #[error("invalid type byte: {0}")]
    InvalidTypeByte(u8),
    #[error("invalid integer")]
    InvalidInteger,
    #[error("invalid length")]
    InvalidLength,
    #[error("invalid boolean")]
    InvalidBoolean,
    #[error("invalid float")]
    InvalidFloat,
}
