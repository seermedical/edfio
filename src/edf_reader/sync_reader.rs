//! Read an EDF file synchronously

use std::io::Error;
use std::convert::TryInto;
use pyo3::prelude::*;
use neon::prelude::*;

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
