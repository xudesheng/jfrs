use crate::reader::type_descriptor::TypeDescriptor;
use crate::reader::value_descriptor::ValueDescriptor;
use crate::reader::{Chunk, Error, HeapByteStream, Result};
use crate::{EVENT_TYPE_CONSTANT_POOL, EVENT_TYPE_METADATA};
#[derive(Debug)]
pub struct Event<'a> {
    pub byte_offset: u64,
    pub class: &'a TypeDescriptor,
    pub(crate) chunk: &'a Chunk,
    pub(crate) value: ValueDescriptor,
}

impl<'a> Event<'a> {
    pub fn value(&'a self) -> Accessor<'a> {
        Accessor {
            chunk: self.chunk,
            value: &self.value,
        }
    }
}

pub struct Accessor<'a> {
    chunk: &'a Chunk,
    pub value: &'a ValueDescriptor,
}

impl<'a> Accessor<'a> {
    pub fn new(chunk: &'a Chunk, value: &'a ValueDescriptor) -> Self {
        Self { chunk, value }
    }

    pub fn get_field(&self, name: &str) -> Option<Self> {
        self.value.get_field(name, self.chunk).map(|v| Self {
            chunk: self.chunk,
            value: v,
        })
    }

    pub fn get_field_raw(&self, name: &str) -> Option<Self> {
        self.value.get_field_raw(name, self.chunk).map(|v| Self {
            chunk: self.chunk,
            value: v,
        })
    }

    pub fn resolve(self) -> Option<Self> {
        match self.value {
            ValueDescriptor::ConstantPool {
                class_id,
                constant_index,
            } => self
                .chunk
                .constant_pool
                .get(class_id, constant_index)
                .map(|v| Self {
                    chunk: self.chunk,
                    value: v,
                }),
            _ => Some(self),
        }
    }

    pub fn as_iter(self) -> Option<impl Iterator<Item = Accessor<'a>>> {
        let array = match self.value {
            ValueDescriptor::Array(a) => a,
            ValueDescriptor::ConstantPool {
                class_id,
                constant_index,
            } => match self.chunk.constant_pool.get(class_id, constant_index) {
                Some(ValueDescriptor::Array(a)) => a,
                _ => return None,
            },
            _ => return None,
        };
        Some(array.iter().map(|v| Accessor {
            value: v,
            chunk: self.chunk,
        }))
    }
}

pub struct EventIterator<'a, 'b> {
    chunk: &'a Chunk,
    stream: &'b mut HeapByteStream,
    offset: u64,
}

impl<'a, 'b> EventIterator<'a, 'b> {
    pub fn new(chunk: &'a Chunk, stream: &'b mut HeapByteStream) -> Self {
        Self {
            chunk,
            stream,
            offset: 0,
        }
    }

    pub fn seek(&mut self, offset: u64) {
        self.offset = offset;
    }

    fn internal_next(&mut self) -> Result<Option<Event<'a>>> {
        let end_offset = self.chunk.header.chunk_body_size();

        while self.offset < end_offset {
            self.stream
                .seek(self.chunk.header.body_start_offset() + self.offset)?;
            let event_offset = self.offset;

            let size = self.stream.read_i32()?;
            let event_type = self.stream.read_i64()?;
            self.offset += size as u64;

            match event_type {
                EVENT_TYPE_METADATA | EVENT_TYPE_CONSTANT_POOL => {}
                _ => {
                    let type_desc = self
                        .chunk
                        .metadata
                        .type_pool
                        .get(event_type)
                        .ok_or(Error::ClassNotFound(event_type))?;
                    let value =
                        ValueDescriptor::try_new(self.stream, event_type, &self.chunk.metadata)?;

                    return Ok(Some(Event {
                        byte_offset: event_offset,
                        class: type_desc,
                        chunk: self.chunk,
                        value,
                    }));
                }
            }
        }
        Ok(None)
    }
}

impl<'a, 'b> Iterator for EventIterator<'a, 'b> {
    type Item = Result<Event<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.internal_next() {
            Ok(Some(e)) => Some(Ok(e)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}
