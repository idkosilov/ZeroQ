mod errors;
mod mpmc_queue;
mod py_queue;
mod shmem_wrapper;

use crate::errors::{
    FailedCreateSharedMemory, FailedOpenSharedMemory, InvalidParameters,
    QueueClosed, QueueEmpty, QueueFull,
};
use pyo3::prelude::*;

#[pymodule]
fn fastqueue(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<py_queue::Queue>()?;
    m.add("InvalidParameters", m.py().get_type::<InvalidParameters>())?;
    m.add("QueueEmpty", m.py().get_type::<QueueEmpty>())?;
    m.add("QueueFull", m.py().get_type::<QueueFull>())?;
    m.add("QueueClosed", m.py().get_type::<QueueClosed>())?;
    m.add(
        "FailedCreateSharedMemory",
        m.py().get_type::<FailedCreateSharedMemory>(),
    )?;
    m.add(
        "FailedOpenSharedMemory",
        m.py().get_type::<FailedOpenSharedMemory>(),
    )?;
    Ok(())
}
