use std::collections::HashSet;

use pyo3::{create_exception, prelude::*};

create_exception!(gram_tools, AnyhowError, pyo3::exceptions::PyException);

/// A Python module implemented in Rust.
#[pymodule]
fn gram_pytools(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_mentioned, m)?)?;
    Ok(())
}

#[pyfunction]
pub fn get_mentioned(msg: &str) -> PyResult<(HashSet<String>, HashSet<i64>)> {
    gram_core::mention::get_mentioned(msg).map_err(|e| crate::AnyhowError::new_err(e.to_string()))
}
