pub mod codec;
pub mod error;
pub mod intern;
pub mod object;

pub use codec::Codec;
pub use error::Error;
pub use intern::{InternContext, InternPtr};
pub use object::{EncodedCustomType, InternValue, NamespaceRef, Object};

#[cfg(test)]
mod tests;
