mod mpmc_queue;

use crate::mpmc_queue::MpmcQueueError;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use shared_memory::*;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

impl From<MpmcQueueError> for PyErr {
    fn from(error: MpmcQueueError) -> Self {
        match error {
            MpmcQueueError::InvalidSourceLength { expected, actual } => {
                PyValueError::new_err(format!(
                    "Invalid source length: expected {}, got {}",
                    expected, actual
                ))
            },
            MpmcQueueError::InvalidDestinationLength { expected, actual } => {
                PyValueError::new_err(format!(
                    "Invalid destination length: expected {}, got {}",
                    expected, actual
                ))
            },
            MpmcQueueError::QueueFull => {
                PyRuntimeError::new_err("Queue is full")
            },
            MpmcQueueError::QueueEmpty => {
                PyRuntimeError::new_err("Queue is empty")
            },
            MpmcQueueError::BufferTooSmall { required, provided } => {
                PyRuntimeError::new_err(format!(
                    "Buffer too small: required {}, provided {}",
                    required, provided
                ))
            },
            MpmcQueueError::BufferMisaligned { expected, actual } => {
                PyRuntimeError::new_err(format!(
                    "Buffer misaligned: expected {}, actual {}",
                    expected, actual
                ))
            },
            MpmcQueueError::BufferSizeNotPowerOfTwo { actual } => {
                PyRuntimeError::new_err(format!(
                    "Buffer size must be a power of two, got {}",
                    actual
                ))
            },
        }
    }
}

/// A wrapper around Shmem to allow it to be sent between threads.
///
/// # Safety
/// We use `unsafe impl` because shared_memory::Shmem is not automatically
/// Send/Sync. This is safe provided that the shared memory is accessed only
/// through atomics and proper synchronization.
struct ShmemWrapper(Shmem);
unsafe impl Send for ShmemWrapper {}
unsafe impl Sync for ShmemWrapper {}

impl ShmemWrapper {
    fn _shmem(&self) -> &Shmem {
        &self.0
    }
}

/// The Queue class provides a Python interface to the MPMC queue in shared memory.
/// The underlying MPMC queue stores its parameters (element size, capacity) in its header.
#[pyclass]
struct Queue {
    shared_mem: Option<ShmemWrapper>,
    /// The underlying MPMC queue, whose header holds the parameters.
    queue: mpmc_queue::MpmcQueueOnBuffer<'static>,
    closed: Arc<AtomicBool>,
}

#[pymethods]
impl Queue {
    #[new]
    #[pyo3(signature = (name, element_size=None, capacity=None, create=true))]
    fn new(
        name: String,
        element_size: Option<usize>,
        capacity: Option<usize>,
        create: bool,
    ) -> PyResult<Self> {
        // Determine queue parameters.
        let (elem_size, cap) = if create {
            (
                element_size.ok_or_else(|| {
                    PyValueError::new_err(
                        "element_size required when create=true",
                    )
                })?,
                capacity.ok_or_else(|| {
                    PyValueError::new_err("capacity required when create=true")
                })?,
            )
        } else {
            // Open shared memory temporarily to read the header.
            let shmem_temp =
                ShmemConf::new().os_id(&name).open().map_err(|e| {
                    PyRuntimeError::new_err(format!(
                        "Failed to open shared memory '{}': {}",
                        name, e
                    ))
                })?;
            let base_ptr = shmem_temp.as_ptr() as usize;
            let header_ptr = base_ptr as *const mpmc_queue::MpmcQueueHeader;
            let header = unsafe { &*header_ptr };
            (header.element_size, header.buffer_mask + 1)
        };

        let required_size = mpmc_queue::compute_required_size(elem_size, cap);

        let shmem = if create {
            ShmemConf::new()
                .os_id(&name)
                .size(required_size)
                .create()
                .map_err(|e| {
                    PyRuntimeError::new_err(format!(
                        "Failed to create shared memory '{}': {}",
                        name, e
                    ))
                })?
        } else {
            ShmemConf::new().os_id(&name).open().map_err(|e| {
                PyRuntimeError::new_err(format!(
                    "Failed to open shared memory '{}': {}",
                    name, e
                ))
            })?
        };

        let buf_len = shmem.len();
        let buf_ptr = shmem.as_ptr() as *mut MaybeUninit<u8>;
        let buf_slice =
            unsafe { std::slice::from_raw_parts_mut(buf_ptr, buf_len) };

        // Initialize (or attach to) the queue in shared memory.
        let queue = unsafe {
            mpmc_queue::MpmcQueueOnBuffer::init_on_buffer(
                buf_slice, elem_size, cap, create,
            )?
        };

        let queue_static: mpmc_queue::MpmcQueueOnBuffer<'static> =
            unsafe { std::mem::transmute(queue) };

        Ok(Self {
            shared_mem: Some(ShmemWrapper(shmem)),
            queue: queue_static,
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    fn check_active(&self) -> PyResult<()> {
        if self.closed.load(Ordering::Relaxed) {
            Err(PyRuntimeError::new_err("Queue is closed"))
        } else {
            Ok(())
        }
    }

    fn put_nowait(
        &self,
        item: &[u8],
    ) -> PyResult<()> {
        self.check_active()?;
        if item.len() != self.queue.header().element_size {
            return Err(PyValueError::new_err(format!(
                "Data size {} does not match element size {}",
                item.len(),
                self.queue.header().element_size
            )));
        }
        self.queue.enqueue(item)?;
        Ok(())
    }

    fn get_nowait(&self) -> PyResult<Vec<u8>> {
        self.check_active()?;
        let mut buf = vec![0u8; self.queue.header().element_size];
        self.queue.dequeue(&mut buf)?;
        Ok(buf)
    }

    fn __len__(&self) -> PyResult<usize> {
        self.check_active()?;
        let head = self.queue.header().dequeue_pos.load(Ordering::Acquire);
        let tail = self.queue.header().enqueue_pos.load(Ordering::Acquire);
        Ok(tail.saturating_sub(head))
    }

    fn __bool__(&self) -> PyResult<bool> {
        Ok(self.__len__()? > 0)
    }

    fn full(&self) -> PyResult<bool> {
        self.check_active()?;
        Ok(self.__len__()? >= self.queue.header().buffer_mask + 1)
    }

    fn empty(&self) -> PyResult<bool> {
        self.check_active()?;
        Ok(self.__len__()? == 0)
    }

    #[getter]
    fn element_size(&self) -> PyResult<usize> {
        self.check_active()?;
        Ok(self.queue.header().element_size)
    }

    #[getter]
    fn maxsize(&self) -> PyResult<usize> {
        self.check_active()?;
        Ok(self.queue.header().buffer_mask + 1)
    }
}

impl Drop for Queue {
    fn drop(&mut self) {
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        self.closed.store(true, Ordering::Relaxed);
        self.shared_mem.take();
    }
}

/// The fastqueue module implementation.
#[pymodule]
fn fastqueue(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Queue>()?;
    Ok(())
}
