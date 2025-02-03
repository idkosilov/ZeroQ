use crate::mpmc_queue::MpmcQueueError;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

// Define custom Python exceptions that map Rust errors to Python-friendly errors.
// These exceptions allow the Rust library to raise meaningful errors in Python.
pyo3::create_exception!(zeroq, Empty, PyRuntimeError);
pyo3::create_exception!(zeroq, Full, PyRuntimeError);

/// Implements automatic conversion from `MpmcQueueError` to `PyErr`,
/// allowing Rust queue errors to be seamlessly translated into Python exceptions.
impl From<MpmcQueueError> for PyErr {
    fn from(error: MpmcQueueError) -> Self {
        match error {
            // Handle errors related to invalid input sizes.
            MpmcQueueError::InvalidSourceLength { expected, actual } => {
                PyValueError::new_err(format!(
                    "Invalid source length: expected {}, got {}",
                    expected, actual
                ))
            }
            MpmcQueueError::InvalidDestinationLength { expected, actual } => {
                PyValueError::new_err(format!(
                    "Invalid destination length: expected {}, got {}",
                    expected, actual
                ))
            }

            // Handle queue capacity errors.
            MpmcQueueError::QueueFull => Full::new_err("Queue is full"),
            MpmcQueueError::QueueEmpty => Empty::new_err("Queue is empty"),

            // Handle memory-related errors.
            MpmcQueueError::BufferTooSmall { required, provided } => {
                PyValueError::new_err(format!(
                    "Buffer too small: required {}, provided {}",
                    required, provided
                ))
            }
            MpmcQueueError::BufferMisaligned { expected, actual } => {
                PyValueError::new_err(format!(
                    "Buffer misaligned: expected {}, actual {}",
                    expected, actual
                ))
            }
            MpmcQueueError::BufferSizeNotPowerOfTwo { actual } => PyValueError::new_err(format!(
                "Buffer size must be a power of two, got {}",
                actual
            )),
        }
    }
}
