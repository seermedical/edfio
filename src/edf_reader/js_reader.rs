//! Read an EDF file synchronously

use std::io::{Error, ErrorKind};
use std::convert::TryInto;
use neon::prelude::*;

use std::fs::File;
use buffered_offset_reader::{BufOffsetReader, OffsetReadMut};

use crate::edf_reader::parser::Parser;
use chrono::prelude::*;
use chrono::Utc;

pub const EDF_HEADER_BYTE_SIZE: u64 = 256;

#[derive(Serialize, Deserialize, Debug,Clone,PartialEq)]
pub struct EDFChannel {
    pub label: String,                         // 16 ascii
    pub transducter_type: String,              // 80 ascii
    pub physical_dimension: String,            // 8 ascii
    pub physical_minimum: f32,                 // 8 ascii
    pub physical_maximum: f32,                 // 8 ascii
    pub digital_minimum: i64,                  // 8 ascii
    pub digital_maximum: i64,                  // 8 ascii
    pub prefiltering: String,                  // 80 ascii
    pub number_of_samples_in_data_record: u64, // 8 ascii
    pub scale_factor: f32,
}

/**
 * EDFHeader :
 *  - 256 bytes of common metadata
 *  - NumberOfChannels * channel metadata = N * 256 bytes
 */
#[derive(Serialize, Deserialize, Debug,Clone,PartialEq)]
pub struct EDFHeader {
    pub file_version: String,
    pub local_patient_identification: String,
    pub local_recording_identification: String,
    pub start_date: String,
    pub start_time: String,
    pub record_start_time_in_ms: i64,
    pub byte_size_header: u64,
    pub number_of_blocks: u64,
    pub block_duration: u64,
    pub number_of_signals: u64,
    pub channels: Vec<EDFChannel>,
}

impl EDFHeader {
    pub fn build_general_header(data: Vec<u8>) -> EDFHeader {
        let mut parser: Parser = Parser::new(data);

        let mut edf_header = EDFHeader {
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
            .map(|v| EDFChannel {
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


fn check_bounds(start_time: u64, duration: u64, edf_header: &EDFHeader) -> Result<(), Error> {
    if start_time + duration > edf_header.block_duration * edf_header.number_of_blocks {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Window is out of bounds",
        ));
    } else {
        Ok(())
    }
}

pub struct SyncEDFReader {
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

impl SyncEDFReader {
    fn new(filename: String) -> Self {
        SyncEDFReader {
            filename,
        }
    }

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
        check_bounds(offset_ms, duration_ms, &self.header().unwrap())?;

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


pub fn read_edf(mut cx: FunctionContext) -> JsResult<JsObject> {
    let filename = cx.argument::<JsString>(0)?.value(&mut cx);
    let reader = SyncEDFReader {
        filename,
    };

    let exported_reader = JsObject::new(&mut cx);
    let js_filename = cx.string(&reader.filename);
    exported_reader.set(&mut cx, "filename", js_filename).unwrap();

    let header = reader.header().unwrap();
    let exported_header = JsObject::new(&mut cx);

    let file_version = cx.string(header.file_version);
    exported_header.set(&mut cx, "fileVersion", file_version).unwrap();
    let local_patient_identification = cx.string(header.local_patient_identification);
    exported_header.set(&mut cx, "localPatientIdentification", local_patient_identification).unwrap();
    let local_recording_identification = cx.string(header.local_recording_identification);
    exported_header.set(&mut cx, "localRecordingIdentification", local_recording_identification).unwrap();
    let start_date = cx.string(header.start_date);
    exported_header.set(&mut cx, "startDate", start_date).unwrap();
    let start_time = cx.string(header.start_time);
    exported_header.set(&mut cx, "startTime", start_time).unwrap();
    let record_start_time_in_ms = cx.number(header.record_start_time_in_ms as f64);
    exported_header.set(&mut cx, "recordStartTimeInMs", record_start_time_in_ms).unwrap();
    let byte_size_header = cx.number(header.byte_size_header as f64);
    exported_header.set(&mut cx, "byteSizeHeader", byte_size_header).unwrap();
    let number_of_blocks = cx.number(header.number_of_blocks as f64);
    exported_header.set(&mut cx, "numberOfBlocks", number_of_blocks).unwrap();
    let block_duration = cx.number(header.block_duration as f64);
    exported_header.set(&mut cx, "blockDuration", block_duration).unwrap();
    let number_of_signals = cx.number(header.number_of_signals as f64);
    exported_header.set(&mut cx, "numberOfSignals", number_of_signals).unwrap();

    let exported_channels = JsArray::new(&mut cx, header.channels.len() as u32);

    for (i, channel) in header.channels.iter().enumerate() {
        let exported_channel = JsObject::new(&mut cx);

        let label = cx.string(&channel.label);
        exported_channel.set(&mut cx, "label", label).unwrap();
        let transducter_type = cx.string(&channel.transducter_type);
        exported_channel.set(&mut cx, "transducterType", transducter_type).unwrap();
        let physical_dimension = cx.string(&channel.physical_dimension);
        exported_channel.set(&mut cx, "physicalDimension", physical_dimension).unwrap();
        let physical_minimum = cx.number(channel.physical_minimum as f64);
        exported_channel.set(&mut cx, "physicalMinimum", physical_minimum).unwrap();
        let physical_maximum = cx.number(channel.physical_maximum as f64);
        exported_channel.set(&mut cx, "physicalMaximum", physical_maximum).unwrap();
        let digital_minimum = cx.number(channel.digital_minimum as f64);
        exported_channel.set(&mut cx, "digitalMinimum", digital_minimum).unwrap();
        let digital_maximum = cx.number(channel.digital_maximum as f64);
        exported_channel.set(&mut cx, "digitalMaximum", digital_maximum).unwrap();
        let prefiltering = cx.string(&channel.prefiltering);
        exported_channel.set(&mut cx, "prefiltering", prefiltering).unwrap();
        let number_of_samples_in_data_record = cx.number(channel.number_of_samples_in_data_record as f64);
        exported_channel.set(&mut cx, "numberOfSamplesInDataRecord", number_of_samples_in_data_record).unwrap();
        let scale_factor = cx.number(channel.scale_factor as f64);
        exported_channel.set(&mut cx, "scaleFactor", scale_factor).unwrap();

        exported_channels.set(&mut cx, i as u32, exported_channel).unwrap();
    }

    exported_header.set(&mut cx, "channels", exported_channels).unwrap();

    exported_reader.set(&mut cx, "header", exported_header).unwrap();
    Ok(exported_reader)
}
