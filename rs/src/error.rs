use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("msgpack encode error: {0}")]
    Encode(#[from] rmp::encode::ValueWriteError),

    #[error("ndarray-npy error: {0}")]
    NpyRead(#[from] ndarray_npy::ReadNpyError),

    #[error("ndarray-npy error: {0}")]
    Npy(#[from] ndarray_npy::WriteNpyError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[cfg(feature = "polars")]
    #[error("polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    #[error("msgpack decode error: {0}")]
    Decode(#[from] rmp::decode::ValueReadError),

    #[error("msgpack decode error: {0}")]
    DecodeConvert(#[from] rmpv::decode::Error),

    #[error("Unexpected value: {0:?}")]
    UnexpectedValue(rmpv::Value),

    #[error("Unexpected value: {0:?}")]
    UnexpectedValueRef(String),
}

impl From<rmpv::Value> for Error {
    fn from(value: rmpv::Value) -> Self {
        Error::UnexpectedValue(value)
    }
}

impl From<rmpv::ValueRef<'_>> for Error {
    fn from(value: rmpv::ValueRef<'_>) -> Self {
        Error::UnexpectedValueRef(format!("{:?}", value))
    }
}
