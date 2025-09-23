pub mod precompute;

use gram_core::mention;
use pyo3::{create_exception, prelude::*};

create_exception!(gram_tools, AnyhowError, pyo3::exceptions::PyException);

/// A Python module implemented in Rust.
#[pymodule]
fn gram_tools(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(mention::get_mentioned, m)?)?;
    Ok(())
}
