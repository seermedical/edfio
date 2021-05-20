//! Read an EDF file synchronously

use std::io::Error;
use std::convert::TryInto;
use pyo3::prelude::*;

use std::fs::File;
use buffered_offset_reader::{BufOffsetReader, OffsetReadMut};

use crate::edf_reader::model::{EDFHeader, EDF_HEADER_BYTE_SIZE};

#[pyclass(dict)]
pub struct SyncEDFReader {
    #[pyo3(get)]
    pub filename: String
}

impl SyncEDFReader {
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, std::io::Error> {
        let f = File::open(&self.filename)?;
        let mut r = BufOffsetReader::new(f);

        let mut buffer = vec![0u8; length];
        r.read_at(&mut buffer, offset)?;

        Ok(buffer)
    }
}

#[pymethods]
impl SyncEDFReader {
    #[new]
    fn new(filename: String) -> Self {
        SyncEDFReader {
            filename,
        }
    }

    #[getter]
    fn header(&self) -> Result<EDFHeader, Error> {
        let general_header_raw = self.read(0, 256)?;

        let mut edf_header = EDFHeader::build_general_header(general_header_raw);

        let channel_headers_raw = self.read(
            256,
            (edf_header.number_of_signals * EDF_HEADER_BYTE_SIZE).try_into().unwrap(),
        )?;

        edf_header.build_channel_headers(channel_headers_raw);

        Ok(edf_header)
    }

    pub fn read_data_window(
        &self,
        offset_ms: u64, // in mS
        duration_ms: u64,   // in mS
    ) -> Result<Vec<Vec<f32>>, Error> {
        super::check_bounds(offset_ms, duration_ms, &self.header().unwrap())?;

        // calculate the corresponding blocks to get

        let first_block_offset = offset_ms - offset_ms % self.header().unwrap().block_duration;

        let first_block_index = first_block_offset / self.header().unwrap().block_duration;

        let number_of_blocks_to_get =
            (duration_ms as f64 / self.header().unwrap().block_duration as f64).ceil() as u64;

        let offset_bytes = self.header().unwrap().byte_size_header
            + first_block_index * self.header().unwrap().get_size_of_data_block();

        let data;

        // TODO : better handle of errors

        match self.read(
            offset_bytes,
            (number_of_blocks_to_get * self.header().unwrap().get_size_of_data_block()).try_into().unwrap(),
        ) {
            Ok(d) => data = d,
            Err(e) => return Err(e),
        }

        let mut result: Vec<Vec<f32>> = Vec::new();

        for _ in 0..self.header().unwrap().number_of_signals {
            result.push(Vec::new());
        }

        let mut index = 0;

        for _ in 0..number_of_blocks_to_get {
            for (j, channel) in self.header().unwrap().channels.iter().enumerate() {
                for _ in 0..channel.number_of_samples_in_data_record {
                    let sample = super::get_sample(&data, index) as f32;
                    result[j].push(
                        (sample - channel.digital_minimum as f32) * channel.scale_factor
                            + channel.physical_minimum,
                    );
                    index += 1;
                }
            }
        }

        Ok(result)
    }
}
