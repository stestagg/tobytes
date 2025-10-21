mod error;
mod encode;
mod decode;
pub mod table_ns;
use error::Error;

pub use encode::{ToBytes, NamespaceEncodedValue};
pub use decode::FromBytes;
pub use table_ns::{FromTableNs, ToTableNs};

#[cfg(feature = "derive")]
pub use tobytes_derive::{ToBytesDict, FromBytesDict};

pub type ToBytesResult<T> = std::result::Result<T, Error>;

pub trait Namespace {
    fn name() -> &'static str;
}

pub const CUSTOM_TYPE_EXT: i8 = 8;

pub mod prelude {
    pub use crate::{ToBytes, FromBytes, Namespace, NamespaceEncodedValue, ToBytesResult};
    pub use crate::{FromTableNs, ToTableNs};
    #[cfg(feature = "derive")]
    pub use crate::{ToBytesDict, FromBytesDict};
}