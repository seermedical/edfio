#[macro_use]
extern crate serde_derive;

use pyo3::prelude::*;
use neon::prelude::*;

mod edf_reader;

use edf_reader::{sync_reader};

#[pymodule]
fn edfio(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<sync_reader::SyncEDFReader>()?;

    Ok(())
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("readEDF", sync_reader::read_edf)?;
    Ok(())
}