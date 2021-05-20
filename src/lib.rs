#[macro_use]
extern crate serde_derive;

use pyo3::prelude::*;

mod edf_reader;

use edf_reader::{sync_reader, file_reader};

#[pymodule]
fn edfio(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<sync_reader::SyncEDFReader>()?;

    Ok(())
}
