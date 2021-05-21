//! Read an EDF file synchronously

use std::io::{Error, ErrorKind};
use std::convert::TryInto;
use pyo3::prelude::*;

use std::fs::File;
use buffered_offset_reader::{BufOffsetReader, OffsetReadMut};

use crate::edf_reader::parser::Parser;
use chrono::prelude::*;
use chrono::Utc;

pub const EDF_HEADER_BYTE_SIZE: u64 = 256;

#[pyclass(dict)]
#[derive(Serialize, Deserialize, Debug,Clone,PartialEq)]
pub struct PyEDFChannel {
    #[pyo3(get, set)]
    pub label: String,                         // 16 ascii
    #[pyo3(get, set)]
    pub transducter_type: String,              // 80 ascii
    #[pyo3(get, set)]
    pub physical_dimension: String,            // 8 ascii
    #[pyo3(get, set)]
    pub physical_minimum: f32,                 // 8 ascii
    #[pyo3(get, set)]
    pub physical_maximum: f32,                 // 8 ascii
    #[pyo3(get, set)]
    pub digital_minimum: i64,                  // 8 ascii
    #[pyo3(get, set)]
    pub digital_maximum: i64,                  // 8 ascii
    #[pyo3(get, set)]
    pub prefiltering: String,                  // 80 ascii
    #[pyo3(get, set)]
    pub number_of_samples_in_data_record: u64, // 8 ascii
    #[pyo3(get, set)]
    pub scale_factor: f32,
}

/**
 * EDFHeader :
 *  - 256 bytes of common metadata
 *  - NumberOfChannels * channel metadata = N * 256 bytes
 */
#[pyclass(dict)]
#[derive(Serialize, Deserialize, Debug,Clone,PartialEq)]
pub struct PyEDFHeader {
    #[pyo3(get, set)]
    pub file_version: String,
    #[pyo3(get, set)]
    pub local_patient_identification: String,
    #[pyo3(get, set)]
    pub local_recording_identification: String,
    #[pyo3(get, set)]
    pub start_date: String,
    #[pyo3(get, set)]
    pub start_time: String,
    #[pyo3(get, set)]
    pub record_start_time_in_ms: i64,
    #[pyo3(get, set)]
    pub byte_size_header: u64,
    #[pyo3(get, set)]
    pub number_of_blocks: u64,
    #[pyo3(get, set)]
    pub block_duration: u64,
    #[pyo3(get, set)]
    pub number_of_signals: u64,
    #[pyo3(get, set)]
    pub channels: Vec<PyEDFChannel>,
}

impl PyEDFHeader {
    pub fn build_general_header(data: Vec<u8>) -> PyEDFHeader {
        let mut parser: Parser = Parser::new(data);

        let mut edf_header = PyEDFHeader {
            file_version: parser.parse_string(8),
            local_patient_identification: parser.parse_string(80),
            local_recording_identification: parser.parse_string(80),
            start_date: parser.parse_string(8),
            start_time: parser.parse_string(8),
            record_start_time_in_ms: 0,
            byte_size_header: parser.parse_number::<u64>(8),
            number_of_blocks: parser.move_offset(44).parse_number::<u64>(8),
            block_duration: parser.parse_number::<u64>(8) * 1000, // to get in mS
            number_of_signals: parser.parse_number::<u64>(4),
            channels: Vec::new(),
        };

        // TODO : create record_start_time
        edf_header.create_start_time();

        return edf_header;
    }

    pub fn build_channel_headers(&mut self, data: Vec<u8>) {
        // check if given data has the good size

        let mut parser = Parser::new(data);

        let label_list = parser.parse_string_list(self.number_of_signals, 16);
        let transductor_type_list = parser.parse_string_list(self.number_of_signals, 80);
        let physical_dimension_list = parser.parse_string_list(self.number_of_signals, 8);
        let physical_minimum_list = parser.parse_number_list::<f32>(self.number_of_signals, 8);
        let physical_maximum_list = parser.parse_number_list::<f32>(self.number_of_signals, 8);
        let digital_minimum_list = parser.parse_number_list::<isize>(self.number_of_signals, 8);
        let digital_maximum_list = parser.parse_number_list::<isize>(self.number_of_signals, 8);
        let prefiltering_list = parser.parse_string_list(self.number_of_signals, 80);
        let number_of_samples_in_data_record_list =
            parser.parse_number_list::<u64>(self.number_of_signals, 8);

        self.channels = (0..self.number_of_signals as usize)
            .map(|v| PyEDFChannel {
                label: label_list[v].clone(),
                transducter_type: transductor_type_list[v].clone(),
                physical_dimension: physical_dimension_list[v].clone(),
                physical_minimum: physical_minimum_list[v],
                physical_maximum: physical_maximum_list[v],
                digital_minimum: digital_minimum_list[v] as i64,
                digital_maximum: digital_maximum_list[v] as i64,
                prefiltering: prefiltering_list[v].clone(),
                number_of_samples_in_data_record: number_of_samples_in_data_record_list[v],
                scale_factor: (physical_maximum_list[v] - physical_minimum_list[v])
                    / (digital_maximum_list[v] - digital_minimum_list[v]) as f32,
            })
            .collect();
    }

    pub fn get_size_of_data_block(&self) -> u64 {
        self.channels
            .iter()
            .map(|channel| channel.number_of_samples_in_data_record * 2)
            .sum()
    }

    fn create_start_time(&mut self) {
        if self.start_date != "" && self.start_time != "" {
            let get_integers = |s: &String| -> Vec<u32> {
                s.split(".").map(|v| v.parse::<u32>().unwrap()).collect()
            };

            let splitted_date = get_integers(&self.start_date);
            let splitted_time = get_integers(&self.start_time);

            let real_year: i32 = 2000 + splitted_date[2] as i32;

            let date = Utc
                .ymd(real_year, splitted_date[1], splitted_date[0])
                .and_hms(splitted_time[0], splitted_time[1], splitted_time[2]);

            self.record_start_time_in_ms = date.timestamp_millis();
        }
    }
}

fn py_check_bounds(start_time: u64, duration: u64, edf_header: &PyEDFHeader) -> Result<(), Error> {
    if start_time + duration > edf_header.block_duration * edf_header.number_of_blocks {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Window is out of bounds",
        ));
    } else {
        Ok(())
    }
}

#[pyclass(dict)]
pub struct PySyncEDFReader {
    #[pyo3(get)]
    pub filename: String
}

impl PySyncEDFReader {
    fn read(&self, offset: u64, length: usize) -> Result<Vec<u8>, std::io::Error> {
        let f = File::open(&self.filename)?;
        let mut r = BufOffsetReader::new(f);

        let mut buffer = vec![0u8; length];
        r.read_at(&mut buffer, offset)?;

        Ok(buffer)
    }
}

#[pymethods]
impl PySyncEDFReader {
    #[new]
    fn new(filename: String) -> Self {
        PySyncEDFReader {
            filename,
        }
    }

    #[getter]
    fn header(&self) -> Result<PyEDFHeader, Error> {
        let general_header_raw = self.read(0, 256)?;

        let mut edf_header = PyEDFHeader::build_general_header(general_header_raw);

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
        py_check_bounds(offset_ms, duration_ms, &self.header().unwrap())?;

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

