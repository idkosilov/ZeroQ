mod errors;
mod mpmc_queue;
mod py_queue;
mod shmem_wrapper;

use crate::errors::{Empty, Full};
use pyo3::prelude::*;

#[pymodule]
fn zeroq(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<py_queue::Queue>()?;
    m.add("Empty", m.py().get_type::<Empty>())?;
    m.add("Full", m.py().get_type::<Full>())?;
    Ok(())
}
