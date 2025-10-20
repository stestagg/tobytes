use std::collections::HashMap;
use std::convert::TryFrom as _;
use std::io::Cursor;

use rmpv::decode::read_value;
use rmpv::Value;

use crate::error::Error;
use crate::intern::{InternContext, INTERN_TABLE_EXT};
use crate::object::{EncodedCustomType, NamespaceRef, Object};

pub const CUSTOM_TYPE_EXT: i8 = 8;

pub type Namespaces = HashMap<String, Namespace>;

pub enum Namespace {
    Static(HashMap<u32, Box<dyn CustomTypeCodec>>),
    Dynamic(Box<dyn CustomNamespace>),
}

pub trait CustomTypeCodec: Send + Sync {
    fn matches(&self, _obj: &Object) -> bool {
        false
    }

    fn encode(&self, codec: &mut Codec, obj: &Object) -> Result<EncodedCustomType, Error>;

    fn decode(&self, codec: &mut Codec, data: &EncodedCustomType) -> Result<Object, Error>;
}

pub trait CustomNamespace: Send + Sync {
    fn matches(&self, _obj: &Object) -> bool {
        false
    }

    fn encode(&self, codec: &mut Codec, obj: &Object) -> Result<EncodedCustomType, Error>;

    fn decode(&self, codec: &mut Codec, data: &EncodedCustomType) -> Result<Object, Error>;
}

pub struct Codec {
    namespaces: Namespaces,
    intern_context: InternContext,
}

impl Codec {
    pub fn new(namespaces: Option<Namespaces>) -> Self {
        Self {
            namespaces: namespaces.unwrap_or_default(),
            intern_context: InternContext::new(),
        }
    }

    pub fn add_namespace(&mut self, namespace: String, types: Namespace) -> Result<(), Error> {
        if self.namespaces.contains_key(&namespace) {
            return Err(Error::InvalidState);
        }
        self.namespaces.insert(namespace, types);
        Ok(())
    }

    pub fn dumps(&mut self, obj: &Object) -> Result<Vec<u8>, Error> {
        self.intern_context.reset();
        let mut buf = self.encode_object(obj)?;
        buf = self.intern_context.finalize_encoding(buf)?;
        Ok(buf)
    }

    pub fn loads(&mut self, data: &[u8]) -> Result<Object, Error> {
        self.intern_context.reset();
        let mut cursor = Cursor::new(data);
        let value = read_value(&mut cursor)?;
        self.decode_value(value)
    }

    fn encode_object(&mut self, obj: &Object) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        self.write_object(&mut buf, obj)?;
        Ok(buf)
    }

    fn write_object(&mut self, buf: &mut Vec<u8>, obj: &Object) -> Result<(), Error> {
        match obj {
            Object::Nil => {
                rmp::encode::write_nil(buf)?;
            }
            Object::Boolean(value) => {
                rmp::encode::write_bool(buf, *value)?;
            }
            Object::Integer(value) => {
                if let Some(v) = value.as_u64() {
                    rmp::encode::write_uint(buf, v)?;
                } else if let Some(v) = value.as_i64() {
                    rmp::encode::write_sint(buf, v)?;
                } else {
                    return Err(Error::IntegerOutOfRange);
                }
            }
            Object::F32(value) => {
                rmp::encode::write_f32(buf, *value)?;
            }
            Object::F64(value) => {
                rmp::encode::write_f64(buf, *value)?;
            }
            Object::String(value) => {
                rmp::encode::write_str(buf, value)?;
            }
            Object::Binary(value) => {
                rmp::encode::write_bin_len(buf, value.len() as u32)?;
                buf.extend_from_slice(value);
            }
            Object::Array(values) => {
                rmp::encode::write_array_len(buf, values.len() as u32)?;
                for value in values {
                    self.write_object(buf, value)?;
                }
            }
            Object::Map(entries) => {
                rmp::encode::write_map_len(buf, entries.len() as u32)?;
                for (key, value) in entries {
                    self.write_object(buf, key)?;
                    self.write_object(buf, value)?;
                }
            }
            Object::Ext(code, data) => {
                rmp::encode::write_ext_meta(buf, data.len() as u32, *code)?;
                buf.extend_from_slice(data);
            }
            Object::Custom(custom) => {
                let payload = self.encode_custom_payload(custom)?;
                rmp::encode::write_ext_meta(buf, payload.len() as u32, CUSTOM_TYPE_EXT)?;
                buf.extend_from_slice(&payload);
            }
            Object::Intern(intern) => {
                let encoded = self.encode_object(intern.value())?;
                let ext = self.intern_context.intern_with_encoded(intern.clone(), encoded)?;
                ext.write(buf)?;
            }
        }
        Ok(())
    }

    fn encode_custom_payload(&mut self, custom: &EncodedCustomType) -> Result<Vec<u8>, Error> {
        let mut payload = Vec::new();
        match &custom.namespace {
            NamespaceRef::Name(name) => {
                rmp::encode::write_str(&mut payload, name)?;
            }
            NamespaceRef::Id(id) => {
                rmp::encode::write_uint(&mut payload, *id as u64)?;
            }
        }
        rmp::encode::write_uint(&mut payload, custom.type_id as u64)?;
        rmp::encode::write_bin_len(&mut payload, custom.data.len() as u32)?;
        payload.extend_from_slice(&custom.data);
        Ok(payload)
    }

    fn decode_value(&mut self, value: Value) -> Result<Object, Error> {
        match value {
            Value::Nil => Ok(Object::Nil),
            Value::Boolean(value) => Ok(Object::Boolean(value)),
            Value::Integer(value) => Ok(Object::Integer(value)),
            Value::F32(value) => Ok(Object::F32(value)),
            Value::F64(value) => Ok(Object::F64(value)),
            Value::String(value) => {
                let value = value.into_str().ok_or(Error::InvalidUtf8)?;
                Ok(Object::String(value))
            }
            Value::Binary(value) => Ok(Object::Binary(value)),
            Value::Array(values) => {
                let mut result = Vec::with_capacity(values.len());
                for value in values {
                    result.push(self.decode_value(value)?);
                }
                Ok(Object::Array(result))
            }
            Value::Map(entries) => {
                let mut result = Vec::with_capacity(entries.len());
                for (key, value) in entries {
                    let key = self.decode_value(key)?;
                    let value = self.decode_value(value)?;
                    result.push((key, value));
                }
                Ok(Object::Map(result))
            }
            Value::Ext(code, data) => self.handle_ext(code, data),
        }
    }

    fn handle_ext(&mut self, code: i8, data: Vec<u8>) -> Result<Object, Error> {
        match code {
            INTERN_TABLE_EXT => {
                if self.intern_context.is_decoding() {
                    self.decode_intern_reference(data)
                } else {
                    self.decode_intern_table(data)
                }
            }
            CUSTOM_TYPE_EXT => self.decode_custom_type(data),
            _ => Ok(Object::Ext(code, data)),
        }
    }

    fn decode_intern_reference(&mut self, data: Vec<u8>) -> Result<Object, Error> {
        let mut cursor = Cursor::new(&data);
        let value = read_value(&mut cursor)?;
        let index = value.as_u64().ok_or(Error::InvalidInternReferencePayload)? as usize;
        self.intern_context.resolve_reference(index)
    }

    fn decode_intern_table(&mut self, data: Vec<u8>) -> Result<Object, Error> {
        self.intern_context.start_decoding()?;
        let result = self.decode_intern_table_inner(data);
        self.intern_context.finish_decoding();
        result
    }

    fn decode_intern_table_inner(&mut self, data: Vec<u8>) -> Result<Object, Error> {
        let mut cursor = Cursor::new(&data);
        let interned_objects_value = read_value(&mut cursor)?;
        let consumed = cursor.position() as usize;
        let entries = match interned_objects_value {
            Value::Array(values) => values,
            _ => return Err(Error::InvalidInternTable),
        };

        for value in entries {
            let decoded = self.decode_value(value)?;
            self.intern_context.push_decoded(decoded)?;
        }

        let remaining = data.get(consumed..).ok_or(Error::InvalidInternTable)?;
        if remaining.is_empty() {
            return Err(Error::InvalidInternTable);
        }
        let mut cursor = Cursor::new(remaining);
        let value = read_value(&mut cursor)?;
        self.decode_value(value)
    }

    fn decode_custom_type(&mut self, data: Vec<u8>) -> Result<Object, Error> {
        let mut cursor = Cursor::new(&data);
        let namespace_value = read_value(&mut cursor)?;
        let namespace = match namespace_value {
            Value::String(value) => {
                let value = value.into_str().ok_or(Error::InvalidUtf8)?;
                NamespaceRef::Name(value)
            }
            Value::Integer(value) => {
                let raw = value.as_u64().ok_or(Error::InvalidCustomNamespace)?;
                let id = u32::try_from(raw).map_err(|_| Error::InvalidCustomNamespace)?;
                NamespaceRef::Id(id)
            }
            _ => return Err(Error::InvalidCustomNamespace),
        };

        let type_id_value = read_value(&mut cursor)?;
        let type_id_raw = type_id_value.as_u64().ok_or(Error::InvalidCustomTypeId)?;
        let type_id = u32::try_from(type_id_raw).map_err(|_| Error::InvalidCustomTypeId)?;

        let consumed = cursor.position() as usize;
        let payload = data
            .get(consumed..)
            .map(|slice| slice.to_vec())
            .unwrap_or_default();

        Ok(Object::Custom(EncodedCustomType {
            namespace,
            type_id,
            data: payload,
        }))
    }
}

impl Default for Codec {
    fn default() -> Self {
        Self::new(None)
    }
}
