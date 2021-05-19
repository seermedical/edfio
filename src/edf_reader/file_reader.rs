//! Contains traits/implementations for reading a file in a sync or async manner.

use std::fs::File;
use futures::Future;
use buffered_offset_reader::{BufOffsetReader, OffsetReadMut};

/**
 * An synchronous file reader
 */
pub trait SyncFileReader {
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, std::io::Error>;
}

/**
 * An asynchronous file reader (returns futures)
 */
pub trait AsyncFileReader {
    fn read_async(
        &self,
        offset: u64,
        length: usize,
    ) -> Box<dyn Future<Item = Vec<u8>, Error = std::io::Error> + Send>;
}

pub struct FileReader {
    pub filename: String
}

impl SyncFileReader for FileReader {
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, std::io::Error> {
        let f = File::open(&self.filename)?;
        let mut r = BufOffsetReader::new(f);

        let mut buffer = vec![0u8; length];
        r.read_at(&mut buffer, offset)?;

        Ok(buffer)
    }
}
