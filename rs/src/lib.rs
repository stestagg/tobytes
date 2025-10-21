mod decode;
mod encode;
mod error;
pub mod table_ns;
use error::Error;

pub use decode::FromBytes;
pub use encode::{NamespaceEncodedValue, ToBytes};
pub use table_ns::{FromTableNs, ToTableNs};

#[cfg(feature = "derive")]
pub use tobytes_derive::{FromBytesDict, ToBytesDict};

pub type ToBytesResult<T> = std::result::Result<T, Error>;

pub trait Namespace {
    fn name() -> &'static str;
}

pub const CUSTOM_TYPE_EXT: i8 = 8;

pub mod prelude {
    pub use crate::{FromBytes, Namespace, NamespaceEncodedValue, ToBytes, ToBytesResult};
    #[cfg(feature = "derive")]
    pub use crate::{FromBytesDict, ToBytesDict};
    pub use crate::{FromTableNs, ToTableNs};
}
