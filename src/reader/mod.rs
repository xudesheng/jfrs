use crate::reader::byte_reader::ByteReader;
use crate::{Version, MAGIC};
use std::io::{Read, Seek};

mod byte_reader;
mod v1;

#[derive(Debug)]
pub enum Error {
    InvalidFormat,
    InvalidString,
    UnsupportedVersion(Version),
    IoError(std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Chunk {}

pub struct JfrReader<R> {
    inner: R,
}

impl<R> JfrReader<R>
where
    R: Read + Seek,
{
    pub fn read_chunk(&mut self) -> Result<Chunk> {
        let mut magic = [0u8; 4];
        self.inner.read_exact(&mut magic).map_err(Error::IoError)?;

        if magic != MAGIC {
            return Err(Error::InvalidFormat);
        }

        let version = Version {
            major: ByteReader::Raw.read_i16(&mut self.inner)?,
            minor: ByteReader::Raw.read_i16(&mut self.inner)?,
        };

        match version {
            crate::VERSION_1 | crate::VERSION_2 => v1::ChunkReader::wrap(&mut self.inner).read(),
            _ => Err(Error::UnsupportedVersion(version)),
        }
    }

    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::path::PathBuf;

    #[test]
    fn test_read_chunk() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-data/profiler-wall.jfr");
        let mut reader = JfrReader::new(File::open(path).unwrap());

        assert!(reader.read_chunk().is_err());
    }
}
