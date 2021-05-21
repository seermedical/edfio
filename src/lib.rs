#[macro_use]
extern crate serde_derive;

use pyo3::prelude::*;
use neon::prelude::*;

mod edf_reader;

// Comment out this function and all code in src/edf_reader/python_reader
// before building the NPM package via Neon, because Node can't make sense
// of the Python bindings.
#[pymodule]
fn edfio(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<edf_reader::python_reader::PySyncEDFReader>()?;

    Ok(())
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("readEDF", edf_reader::js_reader::read_edf)?;
    Ok(())
}