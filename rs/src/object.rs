use std::fmt;
use std::sync::Arc;

use rmpv::Integer;

#[derive(Clone, PartialEq)]
pub enum Object {
    Nil,
    Boolean(bool),
    Integer(Integer),
    F32(f32),
    F64(f64),
    String(String),
    Binary(Vec<u8>),
    Array(Vec<Object>),
    Map(Vec<(Object, Object)>),
    Ext(i8, Vec<u8>),
    Custom(EncodedCustomType),
    Intern(InternValue),
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Object::Nil => f.write_str("Nil"),
            Object::Boolean(value) => f.debug_tuple("Boolean").field(value).finish(),
            Object::Integer(value) => f.debug_tuple("Integer").field(value).finish(),
            Object::F32(value) => f.debug_tuple("F32").field(value).finish(),
            Object::F64(value) => f.debug_tuple("F64").field(value).finish(),
            Object::String(value) => f.debug_tuple("String").field(value).finish(),
            Object::Binary(value) => f
                .debug_tuple("Binary")
                .field(&format!("len={}", value.len()))
                .finish(),
            Object::Array(values) => f.debug_tuple("Array").field(values).finish(),
            Object::Map(entries) => f.debug_tuple("Map").field(entries).finish(),
            Object::Ext(code, data) => f
                .debug_struct("Ext")
                .field("code", code)
                .field("len", &data.len())
                .finish(),
            Object::Custom(custom) => f.debug_tuple("Custom").field(custom).finish(),
            Object::Intern(intern) => f.debug_tuple("Intern").field(intern).finish(),
        }
    }
}

impl From<bool> for Object {
    fn from(value: bool) -> Self {
        Object::Boolean(value)
    }
}

impl From<i64> for Object {
    fn from(value: i64) -> Self {
        Object::Integer(Integer::from(value))
    }
}

impl From<u64> for Object {
    fn from(value: u64) -> Self {
        Object::Integer(Integer::from(value))
    }
}

impl From<f32> for Object {
    fn from(value: f32) -> Self {
        Object::F32(value)
    }
}

impl From<f64> for Object {
    fn from(value: f64) -> Self {
        Object::F64(value)
    }
}

impl From<String> for Object {
    fn from(value: String) -> Self {
        Object::String(value)
    }
}

impl From<&str> for Object {
    fn from(value: &str) -> Self {
        Object::String(value.to_owned())
    }
}

impl From<Vec<u8>> for Object {
    fn from(value: Vec<u8>) -> Self {
        Object::Binary(value)
    }
}

impl Object {
    pub fn map(entries: Vec<(Object, Object)>) -> Self {
        Object::Map(entries)
    }

    pub fn array(values: Vec<Object>) -> Self {
        Object::Array(values)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum NamespaceRef {
    Name(String),
    Id(u32),
}

impl fmt::Debug for NamespaceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamespaceRef::Name(name) => f.debug_tuple("Name").field(name).finish(),
            NamespaceRef::Id(id) => f.debug_tuple("Id").field(id).finish(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedCustomType {
    pub namespace: NamespaceRef,
    pub type_id: u32,
    pub data: Vec<u8>,
}

impl EncodedCustomType {
    pub fn new(namespace: NamespaceRef, type_id: u32, data: Vec<u8>) -> Self {
        Self {
            namespace,
            type_id,
            data,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct InternValue {
    value: Arc<Object>,
    by_identity: bool,
}

impl InternValue {
    pub fn new(value: Object) -> Self {
        Self {
            value: Arc::new(value),
            by_identity: true,
        }
    }

    pub fn by_equality(value: Object) -> Self {
        Self {
            value: Arc::new(value),
            by_identity: false,
        }
    }

    pub fn with_arc(value: Arc<Object>, by_identity: bool) -> Self {
        Self { value, by_identity }
    }

    pub fn value(&self) -> &Object {
        &self.value
    }

    pub fn arc_clone(&self) -> Arc<Object> {
        Arc::clone(&self.value)
    }

    pub fn pointer(&self) -> usize {
        Arc::as_ptr(&self.value) as usize
    }

    pub fn by_identity(&self) -> bool {
        self.by_identity
    }
}

impl fmt::Debug for InternValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InternValue")
            .field("by_identity", &self.by_identity)
            .finish()
    }
}
