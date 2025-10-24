use std::collections::HashSet;

use pyo3::{create_exception, prelude::*};

create_exception!(gram_tools, AnyhowError, pyo3::exceptions::PyException);

/// A Python module implemented in Rust.
#[pymodule]
fn gram_pytools(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(extract_username, m)?)?;
    Ok(())
}

#[pyfunction]
pub fn extract_username(msg: &str) -> PyResult<(HashSet<String>, HashSet<i64>)> {
    gram_core::extract::username::extract_usernames_json(msg)
        .map_err(|e| crate::AnyhowError::new_err(e.to_string()))
}
