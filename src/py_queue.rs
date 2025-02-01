use crate::errors::{
    FailedCreateSharedMemory, FailedOpenSharedMemory, InvalidParameters,
    QueueClosed,
};
use crate::mpmc_queue::MpmcQueueOnBuffer;
use crate::shmem_wrapper::ShmemWrapper;
use pyo3::prelude::*;
use shared_memory::ShmemConf;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[pyclass]
pub struct Queue {
    shared_mem: Option<ShmemWrapper>,
    queue: MpmcQueueOnBuffer<'static>,
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
        let (elem_size, cap) = if create {
            (
                element_size.ok_or_else(|| {
                    InvalidParameters::new_err(
                        "element_size required when create=true",
                    )
                })?,
                capacity.ok_or_else(|| {
                    InvalidParameters::new_err(
                        "capacity required when create=true",
                    )
                })?,
            )
        } else {
            let shmem_temp =
                ShmemConf::new().os_id(&name).open().map_err(|e| {
                    FailedOpenSharedMemory::new_err(format!(
                        "Failed to open shared memory '{}': {}",
                        name, e
                    ))
                })?;
            let base_ptr = shmem_temp.as_ptr() as usize;
            let header_ptr =
                base_ptr as *const crate::mpmc_queue::MpmcQueueHeader;
            let header = unsafe { &*header_ptr };
            (header.element_size, header.buffer_mask + 1)
        };

        let required_size =
            crate::mpmc_queue::compute_required_size(elem_size, cap);

        let shmem = if create {
            ShmemConf::new()
                .os_id(&name)
                .size(required_size)
                .create()
                .map_err(|e| {
                    FailedCreateSharedMemory::new_err(format!(
                        "Failed to create shared memory '{}': {}",
                        name, e
                    ))
                })?
        } else {
            ShmemConf::new().os_id(&name).open().map_err(|e| {
                FailedOpenSharedMemory::new_err(format!(
                    "Failed to open shared memory '{}': {}",
                    name, e
                ))
            })?
        };

        let shmem_wrapper = ShmemWrapper::new(shmem);
        let buf_len = shmem_wrapper.len();
        let buf_ptr = shmem_wrapper.as_ptr() as *mut MaybeUninit<u8>;
        let buf_slice =
            unsafe { std::slice::from_raw_parts_mut(buf_ptr, buf_len) };

        let queue = unsafe {
            MpmcQueueOnBuffer::init_on_buffer(
                buf_slice, elem_size, cap, create,
            )?
        };
        let queue_static: MpmcQueueOnBuffer<'static> =
            unsafe { std::mem::transmute(queue) };

        Ok(Self {
            shared_mem: Some(shmem_wrapper),
            queue: queue_static,
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    fn check_active(&self) -> PyResult<()> {
        if self.closed.load(Ordering::Relaxed) {
            Err(QueueClosed::new_err("Queue is closed"))
        } else {
            Ok(())
        }
    }

    fn put_nowait(
        &self,
        item: &[u8],
    ) -> PyResult<()> {
        self.check_active()?;
        Python::with_gil(|py| py.allow_threads(|| self.queue.enqueue(item)))?;
        Ok(())
    }

    fn get_nowait(&self) -> PyResult<Vec<u8>> {
        self.check_active()?;
        let mut buf = vec![0u8; self.queue.header().element_size];
        Python::with_gil(|py| {
            py.allow_threads(|| self.queue.dequeue(&mut buf))
        })?;
        Ok(buf)
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
