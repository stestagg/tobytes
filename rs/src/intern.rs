use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use crate::error::Error;
use crate::object::{InternValue, Object};

pub const INTERN_TABLE_EXT: i8 = 6;

#[derive(Clone)]
pub struct InternPtr {
    index: usize,
}

impl InternPtr {
    pub fn new(index: usize) -> Self {
        Self { index }
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ext {
    pub ty: i8,
    pub data: Vec<u8>,
}

impl Ext {
    pub fn new(ty: i8, data: Vec<u8>) -> Self {
        Self { ty, data }
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        rmp::encode::write_ext_meta(writer, self.ty, self.data.len() as u32)?;
        writer.write_all(&self.data)?;
        Ok(())
    }
}

enum TableState {
    Encoding(EncodingInternTable),
    Decoding(DecodedInternTable),
}

impl TableState {
    fn as_encoding_mut(&mut self) -> Option<&mut EncodingInternTable> {
        match self {
            TableState::Encoding(table) => Some(table),
            _ => None,
        }
    }

    fn as_decoding(&self) -> Option<&DecodedInternTable> {
        match self {
            TableState::Decoding(table) => Some(table),
            _ => None,
        }
    }

    fn as_decoding_mut(&mut self) -> Option<&mut DecodedInternTable> {
        match self {
            TableState::Decoding(table) => Some(table),
            _ => None,
        }
    }
}

pub struct InternContext {
    state: Option<TableState>,
}

impl InternContext {
    pub fn new() -> Self {
        Self { state: None }
    }

    pub fn reset(&mut self) {
        self.state = None;
    }

    pub fn intern<F>(&mut self, intern_value: InternValue, mut encoder: F) -> Result<Ext, Error>
    where
        F: FnMut(&Object) -> Result<Vec<u8>, Error>,
    {
        let table = self.ensure_encoding_table()?;
        table.intern(intern_value, encoder)
    }

    pub fn finalize_encoding(&mut self, data: Vec<u8>) -> Result<Vec<u8>, Error> {
        match self.state.take() {
            Some(TableState::Encoding(table)) => {
                if table.is_empty() {
                    Ok(data)
                } else {
                    let mut payload = table.get_bytes()?;
                    payload.extend_from_slice(&data);

                    let mut buf = Vec::new();
                    rmp::encode::write_ext_meta(&mut buf, INTERN_TABLE_EXT, payload.len() as u32)?;
                    buf.write_all(&payload)?;
                    Ok(buf)
                }
            }
            Some(TableState::Decoding(_)) => Err(Error::InvalidState),
            None => Ok(data),
        }
    }

    pub fn start_decoding(&mut self) -> Result<(), Error> {
        if self.state.is_some() {
            return Err(Error::NestedInternTable);
        }
        self.state = Some(TableState::Decoding(DecodedInternTable::default()));
        Ok(())
    }

    pub fn push_decoded(&mut self, value: Object) -> Result<(), Error> {
        match self.state.as_mut() {
            Some(TableState::Decoding(table)) => {
                table.entries.push(value);
                Ok(())
            }
            _ => Err(Error::InvalidState),
        }
    }

    pub fn is_decoding(&self) -> bool {
        matches!(self.state, Some(TableState::Decoding(_)))
    }

    pub fn resolve_reference(&self, index: usize) -> Result<Object, Error> {
        match self.state.as_ref() {
            Some(TableState::Decoding(table)) => {
                if let Some(value) = table.entries.get(index) {
                    Ok(value.clone())
                } else {
                    Err(Error::ForwardInternReference {
                        index,
                        size: table.entries.len(),
                    })
                }
            }
            _ => Err(Error::InvalidState),
        }
    }

    pub fn finish_decoding(&mut self) {
        self.state = None;
    }

    fn ensure_encoding_table(&mut self) -> Result<&mut EncodingInternTable, Error> {
        if self.state.is_none() {
            self.state = Some(TableState::Encoding(EncodingInternTable::default()));
        }

        match self.state.as_mut() {
            Some(TableState::Encoding(table)) => Ok(table),
            _ => Err(Error::InvalidState),
        }
    }
}

#[derive(Default)]
struct DecodedInternTable {
    entries: Vec<Object>,
}

#[derive(Default)]
struct EncodingInternTable {
    entries: Vec<Vec<u8>>,
    originals: Vec<Arc<Object>>,
    by_id: HashMap<usize, usize>,
}

impl EncodingInternTable {
    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn get_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut buf = Vec::new();
        rmp::encode::write_array_len(&mut buf, self.entries.len() as u32)?;
        for entry in &self.entries {
            buf.extend_from_slice(entry);
        }
        Ok(buf)
    }

    fn intern<F>(&mut self, intern_value: InternValue, mut encoder: F) -> Result<Ext, Error>
    where
        F: FnMut(&Object) -> Result<Vec<u8>, Error>,
    {
        if intern_value.by_identity() {
            let key = intern_value.pointer();
            if let Some(&idx) = self.by_id.get(&key) {
                return Self::create_reference(idx);
            }
        } else if let Some(idx) = self.find_by_equality(intern_value.value()) {
            return Self::create_reference(idx);
        }

        let encoded = encoder(intern_value.value())?;
        let idx = self.entries.len();
        self.entries.push(encoded);
        let arc = intern_value.arc_clone();
        if intern_value.by_identity() {
            self.by_id.insert(Arc::as_ptr(&arc) as usize, idx);
        }
        self.originals.push(arc);
        Self::create_reference(idx)
    }

    fn find_by_equality(&self, value: &Object) -> Option<usize> {
        self.originals
            .iter()
            .enumerate()
            .find_map(|(idx, candidate)| (candidate.as_ref() == value).then_some(idx))
    }

    fn create_reference(index: usize) -> Result<Ext, Error> {
        let mut buf = Vec::new();
        rmp::encode::write_uint(&mut buf, index as u64)?;
        Ok(Ext::new(INTERN_TABLE_EXT, buf))
    }
}
