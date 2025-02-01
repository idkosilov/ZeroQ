use crate::mpmc_queue::MpmcQueueError;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

pyo3::create_exception!(fastqueue, InvalidParameters, PyValueError);
pyo3::create_exception!(fastqueue, QueueEmpty, PyRuntimeError);
pyo3::create_exception!(fastqueue, QueueFull, PyRuntimeError);
pyo3::create_exception!(fastqueue, QueueClosed, PyRuntimeError);
pyo3::create_exception!(fastqueue, FailedCreateSharedMemory, PyRuntimeError);
pyo3::create_exception!(fastqueue, FailedOpenSharedMemory, PyRuntimeError);

impl From<MpmcQueueError> for PyErr {
    fn from(error: MpmcQueueError) -> Self {
        match error {
            MpmcQueueError::InvalidSourceLength { expected, actual } => {
                InvalidParameters::new_err(format!(
                    "Invalid source length: expected {}, got {}",
                    expected, actual
                ))
            },
            MpmcQueueError::InvalidDestinationLength { expected, actual } => {
                InvalidParameters::new_err(format!(
                    "Invalid destination length: expected {}, got {}",
                    expected, actual
                ))
            },
            MpmcQueueError::QueueFull => QueueFull::new_err("Queue is full"),
            MpmcQueueError::QueueEmpty => QueueEmpty::new_err("Queue is empty"),
            MpmcQueueError::BufferTooSmall { required, provided } => {
                InvalidParameters::new_err(format!(
                    "Buffer too small: required {}, provided {}",
                    required, provided
                ))
            },
            MpmcQueueError::BufferMisaligned { expected, actual } => {
                InvalidParameters::new_err(format!(
                    "Buffer misaligned: expected {}, actual {}",
                    expected, actual
                ))
            },
            MpmcQueueError::BufferSizeNotPowerOfTwo { actual } => {
                InvalidParameters::new_err(format!(
                    "Buffer size must be a power of two, got {}",
                    actual
                ))
            },
        }
    }
}
