use crate::reader::byte_reader::{ByteReader, StringType};
use crate::reader::{Chunk, Error, Result};
use std::io::{Read, Seek, SeekFrom};

const FEATURES_COMPRESSED_INTS: i32 = 1;
const EVENT_TYPE_METADATA: i64 = 0;

pub struct ChunkReader<'a, R>(&'a mut R);

#[derive(Debug)]
struct ChunkHeader {
    chunk_size: i64,
    constant_pool_offset: i64,
    metadata_offset: i64,
    start_time_nanos: i64,
    duration_nanos: i64,
    start_ticks: i64,
    ticks_per_second: i64,
    features: i32,
    body_start_offset: u64,
}

impl ChunkHeader {
    fn is_ints_compressed(&self) -> bool {
        self.features & FEATURES_COMPRESSED_INTS != 0
    }
}

#[derive(Debug)]
struct Metadata {

}

impl<'a, R> ChunkReader<'a, R>
where
    R: Read + Seek,
{
    pub fn wrap(inner: &'a mut R) -> Self {
        Self(inner)
    }

    pub fn read(&mut self) -> Result<Chunk> {
        let header = self.read_header()?;
        println!("header: {:?}", header);

        self.0
            .seek(SeekFrom::Start(header.metadata_offset as u64))
            .map_err(Error::IoError)?;

        let reader = if header.is_ints_compressed() {
            ByteReader::CompressedInts
        } else {
            ByteReader::Raw
        };
        let metadata = self.read_metadata(&reader)?;

        Err(Error::InvalidFormat)
    }

    fn read_header(&mut self) -> Result<ChunkHeader> {
        let reader = ByteReader::Raw;
        
        let header = ChunkHeader {
            chunk_size: reader.read_i64(self.0)?,
            constant_pool_offset: reader.read_i64(self.0)?,
            metadata_offset: reader.read_i64(self.0)?,
            start_time_nanos: reader.read_i64(self.0)?,
            duration_nanos: reader.read_i64(self.0)?,
            start_ticks: reader.read_i64(self.0)?,
            ticks_per_second: reader.read_i64(self.0)?,
            features: reader.read_i32(self.0)?,
            body_start_offset: self.0.stream_position().map_err(Error::IoError)?,
        };

        Ok(header)
    }

    fn read_metadata(&mut self, reader: &ByteReader) -> Result<Metadata> {
        // size
        reader.read_i32(self.0)?;
        if reader.read_i64(self.0)? != EVENT_TYPE_METADATA {
            return Err(Error::InvalidFormat);
        }

        // start time
        reader.read_i64(self.0)?;
        // duration
        reader.read_i64(self.0)?;
        // metadata id
        reader.read_i64(self.0)?;

        let string_count = reader.read_i32(self.0)?;
        let mut strings = Vec::with_capacity(string_count as usize);

        for _ in 0..string_count {
            match reader.read_string(self.0)? {
                StringType::Null => strings.push(None),
                StringType::Empty => strings.push(Some("".to_string())),
                StringType::Raw(s) => strings.push(Some(s)),
                _ => return Err(Error::InvalidString),
            }
        }
        println!("strings: {:?}", strings);

        Err(Error::InvalidFormat)
    }
}
