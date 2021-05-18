#[pymodule]
fn edfiopy(_py: Python, m: &PyModule) => PyResult<()> {
    m.add_class::<Map>()?;
}
