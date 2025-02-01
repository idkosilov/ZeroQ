use crate::mpmc_queue::MpmcQueueError;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

// Define custom Python exceptions that map Rust errors to Python-friendly errors.
// These exceptions allow the Rust library to raise meaningful errors in Python.
pyo3::create_exception!(fastqueue, InvalidParameters, PyValueError);
pyo3::create_exception!(fastqueue, QueueEmpty, PyRuntimeError);
pyo3::create_exception!(fastqueue, QueueFull, PyRuntimeError);
pyo3::create_exception!(fastqueue, QueueClosed, PyRuntimeError);
pyo3::create_exception!(fastqueue, FailedCreateSharedMemory, PyRuntimeError);
pyo3::create_exception!(fastqueue, FailedOpenSharedMemory, PyRuntimeError);

/// Implements automatic conversion from `MpmcQueueError` to `PyErr`,
/// allowing Rust queue errors to be seamlessly translated into Python exceptions.
impl From<MpmcQueueError> for PyErr {
    fn from(error: MpmcQueueError) -> Self {
        match error {
            // Handle errors related to invalid input sizes.
            MpmcQueueError::InvalidSourceLength { expected, actual } => {
                InvalidParameters::new_err(format!(
                    "Invalid source length: expected {}, got {}",
                    expected, actual
                ))
            }
            MpmcQueueError::InvalidDestinationLength { expected, actual } => {
                InvalidParameters::new_err(format!(
                    "Invalid destination length: expected {}, got {}",
                    expected, actual
                ))
            }

            // Handle queue capacity errors.
            MpmcQueueError::QueueFull => QueueFull::new_err("Queue is full"),
            MpmcQueueError::QueueEmpty => QueueEmpty::new_err("Queue is empty"),

            // Handle memory-related errors.
            MpmcQueueError::BufferTooSmall { required, provided } => {
                InvalidParameters::new_err(format!(
                    "Buffer too small: required {}, provided {}",
                    required, provided
                ))
            }
            MpmcQueueError::BufferMisaligned { expected, actual } => {
                InvalidParameters::new_err(format!(
                    "Buffer misaligned: expected {}, actual {}",
                    expected, actual
                ))
            }
            MpmcQueueError::BufferSizeNotPowerOfTwo { actual } => InvalidParameters::new_err(
                format!("Buffer size must be a power of two, got {}", actual),
            ),
        }
    }
}
