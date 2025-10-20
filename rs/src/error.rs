use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("msgpack encode error: {0}")]
    Encode(#[from] rmp::encode::ValueWriteError),

    #[error("msgpack decode error: {0}")]
    Decode(#[from] rmpv::decode::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("integer value out of range for msgpack encoding")]
    IntegerOutOfRange,

    #[error("cannot encode intern table entries while decoding")]
    InvalidState,

    #[error("nested intern tables are not allowed")]
    NestedInternTable,

    #[error("invalid intern table reference: index {index} (table size: {size})")]
    InvalidInternReference { index: usize, size: usize },

    #[error("forward intern reference detected: index {index} (table size: {size})")]
    ForwardInternReference { index: usize, size: usize },

    #[error("invalid intern table payload: expected array of entries")]
    InvalidInternTable,

    #[error("invalid intern reference payload")]
    InvalidInternReferencePayload,

    #[error("invalid custom type namespace value")]
    InvalidCustomNamespace,

    #[error("invalid custom type id value")]
    InvalidCustomTypeId,

    #[error("invalid utf-8 string in message")]
    InvalidUtf8,
}
