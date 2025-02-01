use named_lock::{NamedLock, NamedLockGuard};
use pyo3::exceptions;
use pyo3::exceptions::PyBlockingIOError;
use pyo3::prelude::*;
use shared_memory::*;
use std::mem::{align_of, size_of};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pyo3::create_exception!(rpc_queue, QueueEmpty, exceptions::PyException);
pyo3::create_exception!(rpc_queue, QueueFull, exceptions::PyException);

const HEADER_MAGIC: u64 = 0xDEAD_BEEF_CAFE_BABE;

#[repr(C, align(8))]
struct Header {
    magic: u64,
    capacity: usize,
    element_size: usize,
    mask: usize,
    enqueue_position: AtomicUsize,
    dequeue_position: AtomicUsize,
}

unsafe impl Sync for Header {}

#[pyclass]
struct Queue {
    shared_mem: Option<Shmem>,
    header: *mut Header,
    buffer: *mut u8,
    sequences: *mut AtomicUsize,
    closed: Arc<AtomicBool>,
    lock: NamedLock,
}

unsafe impl Sync for Queue {}
unsafe impl Send for Queue {}

impl Queue {
    fn check_active(&self) -> PyResult<()> {
        if self.closed.load(Ordering::Relaxed) {
            Err(exceptions::PyRuntimeError::new_err("Queue is closed"))
        } else {
            Ok(())
        }
    }

    fn capacity(&self) -> usize {
        unsafe { (*self.header).capacity }
    }

    fn get_element_size(&self) -> usize {
        unsafe { (*self.header).element_size }
    }

    fn mask(&self) -> usize {
        unsafe { (*self.header).mask }
    }

    fn enqueue_pos(&self) -> &AtomicUsize {
        unsafe { &(*self.header).enqueue_position }
    }

    fn dequeue_pos(&self) -> &AtomicUsize {
        unsafe { &(*self.header).dequeue_position }
    }

    fn sequence_at(
        &self,
        index: usize,
    ) -> &AtomicUsize {
        unsafe { &*self.sequences.add(index) }
    }

    fn calculate_layout(
        capacity: usize,
        element_size: usize,
    ) -> PyResult<(usize, usize, usize, usize, usize)> {
        if capacity < 2 || (capacity & (capacity - 1)) != 0 {
            return Err(exceptions::PyValueError::new_err(
                "Capacity must be a power of two and at least 2",
            ));
        }

        let header_size = size_of::<Header>();
        let buffer_size =
            capacity.checked_mul(element_size).ok_or_else(|| {
                exceptions::PyValueError::new_err(
                    "Capacity * element_size overflow",
                )
            })?;
        let sequences_size =
            capacity.checked_mul(size_of::<AtomicUsize>()).ok_or_else(
                || exceptions::PyValueError::new_err("Sequences size overflow"),
            )?;

        let header_align = align_of::<Header>();
        let seq_align = align_of::<AtomicUsize>();

        let total_size = header_align
            .checked_sub(1)
            .and_then(|max| header_size.checked_add(max))
            .and_then(|v| v.checked_add(buffer_size))
            .and_then(|v| {
                seq_align.checked_sub(1).and_then(|max| v.checked_add(max))
            })
            .and_then(|v| v.checked_add(sequences_size))
            .ok_or_else(|| {
                exceptions::PyValueError::new_err("Total size overflow")
            })?;

        Ok((total_size, header_size, buffer_size, sequences_size, seq_align))
    }

    fn with_lock<F, T>(
        &self,
        f: F,
    ) -> PyResult<T>
    where
        F: FnOnce(&NamedLockGuard) -> PyResult<T>,
    {
        let guard = self.lock.lock().map_err(|e| {
            exceptions::PyRuntimeError::new_err(format!("Lock error: {}", e))
        })?;

        f(&guard)
    }
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
        let lock = NamedLock::create(&format!("{}-queue-lock", name)).map_err(
            |e| {
                exceptions::PyRuntimeError::new_err(format!(
                    "Failed to create lock: {}",
                    e
                ))
            },
        )?;

        let _guard = lock.lock().map_err(|e| {
            exceptions::PyRuntimeError::new_err(format!(
                "Failed to acquire lock: {}",
                e
            ))
        })?;

        let shared_mem = if create {
            let element_size = element_size.ok_or_else(|| {
                exceptions::PyValueError::new_err(
                    "element_size required when create=true",
                )
            })?;
            let capacity = capacity.ok_or_else(|| {
                exceptions::PyValueError::new_err(
                    "capacity required when create=true",
                )
            })?;

            let (total_size, _, _, _, _) =
                Self::calculate_layout(capacity, element_size)?;

            ShmemConf::new()
                .os_id(&name)
                .size(total_size)
                .create()
                .map_err(|e| {
                    exceptions::PyRuntimeError::new_err(format!(
                        "Failed to create shared memory '{}': {}",
                        name, e
                    ))
                })?
        } else {
            ShmemConf::new().os_id(&name).open().map_err(|e| {
                exceptions::PyRuntimeError::new_err(format!(
                    "Failed to open shared memory '{}': {}",
                    name, e
                ))
            })?
        };

        let base_ptr = shared_mem.as_ptr() as usize;
        let header_align = align_of::<Header>();
        let header_addr = (base_ptr + header_align - 1) & !(header_align - 1);
        let header_ptr = header_addr as *mut Header;

        let (capacity, element_size, seq_align) = if create {
            (
                capacity.unwrap(),
                element_size.unwrap(),
                align_of::<AtomicUsize>(),
            )
        } else {
            let header = unsafe { &*header_ptr };

            if header.magic != HEADER_MAGIC {
                return Err(exceptions::PyRuntimeError::new_err(
                    "Invalid shared memory header magic",
                ));
            }
            if header.capacity < 2
                || (header.capacity & (header.capacity - 1)) != 0
            {
                return Err(exceptions::PyRuntimeError::new_err(
                    "Invalid capacity in shared memory header",
                ));
            }
            if header.element_size == 0 {
                return Err(exceptions::PyRuntimeError::new_err(
                    "Invalid element_size in shared memory header",
                ));
            }
            if header.mask != header.capacity - 1 {
                return Err(exceptions::PyRuntimeError::new_err(
                    "Invalid mask in shared memory header",
                ));
            }

            (header.capacity, header.element_size, align_of::<AtomicUsize>())
        };

        let (_, header_size, buffer_size, _, _) =
            Self::calculate_layout(capacity, element_size)?;

        let buffer_start = header_addr + header_size;
        let buffer_ptr = buffer_start as *mut u8;

        let buffer_end =
            buffer_start.checked_add(buffer_size).ok_or_else(|| {
                exceptions::PyRuntimeError::new_err("Buffer end overflow")
            })?;

        let sequences_addr = buffer_end
            .checked_add(seq_align - 1)
            .and_then(|v| v.checked_sub(1))
            .ok_or_else(|| {
                exceptions::PyRuntimeError::new_err(
                    "Sequences address overflow",
                )
            })?
            & !(seq_align - 1);

        let sequences_ptr = sequences_addr as *mut AtomicUsize;

        if create {
            unsafe {
                ptr::write(
                    header_ptr,
                    Header {
                        magic: HEADER_MAGIC,
                        capacity,
                        element_size,
                        mask: capacity - 1,
                        enqueue_position: AtomicUsize::new(0),
                        dequeue_position: AtomicUsize::new(0),
                    },
                );

                for i in 0..capacity {
                    let seq = sequences_ptr.add(i);
                    ptr::write(seq, AtomicUsize::new(i));
                }
            }
        }

        Ok(Self {
            shared_mem: Some(shared_mem),
            header: header_ptr,
            buffer: buffer_ptr,
            sequences: sequences_ptr,
            closed: Arc::new(AtomicBool::new(false)),
            lock,
        })
    }

    fn try_put(
        &self,
        data: &[u8],
    ) -> PyResult<bool> {
        self.with_lock(|_guard| {
            self.check_active()?;
            Python::with_gil(|py| {
                py.allow_threads(|| {
                    let element_size = self.get_element_size();
                    if data.len() != element_size {
                        return Err(exceptions::PyValueError::new_err(
                            format!(
                                "Data size {} does not match element size {}",
                                data.len(),
                                element_size
                            ),
                        ));
                    }

                    loop {
                        let pos = self.enqueue_pos().load(Ordering::Acquire);
                        let index = pos & self.mask();
                        let seq =
                            self.sequence_at(index).load(Ordering::Acquire);

                        match seq as isize - pos as isize {
                            0 => {
                                if self
                                    .enqueue_pos()
                                    .compare_exchange_weak(
                                        pos,
                                        pos + 1,
                                        Ordering::AcqRel,
                                        Ordering::Relaxed,
                                    )
                                    .is_ok()
                                {
                                    let offset = index * element_size;
                                    unsafe {
                                        let dest =
                                            std::slice::from_raw_parts_mut(
                                                self.buffer.add(offset),
                                                element_size,
                                            );
                                        dest.copy_from_slice(data);
                                    }

                                    self.sequence_at(index)
                                        .store(pos + 1, Ordering::Release);
                                    return Ok(true);
                                }
                            },
                            diff if diff < 0 => return Ok(false),
                            _ => continue,
                        }
                    }
                })
            })
        })
    }

    fn try_get(&self) -> PyResult<Option<Vec<u8>>> {
        self.with_lock(|_guard| {
            self.check_active()?;
            Python::with_gil(|py| {
                py.allow_threads(|| loop {
                    let pos = self.dequeue_pos().load(Ordering::Acquire);
                    let index = pos & self.mask();
                    let seq = self.sequence_at(index).load(Ordering::Acquire);

                    match seq as isize - (pos + 1) as isize {
                        0 => {
                            if self
                                .dequeue_pos()
                                .compare_exchange_weak(
                                    pos,
                                    pos + 1,
                                    Ordering::AcqRel,
                                    Ordering::Relaxed,
                                )
                                .is_ok()
                            {
                                let offset = index * self.get_element_size();
                                let mut result =
                                    vec![0u8; self.get_element_size()];
                                unsafe {
                                    let src = std::slice::from_raw_parts(
                                        self.buffer.add(offset),
                                        self.get_element_size(),
                                    );
                                    result.copy_from_slice(src);
                                }

                                self.sequence_at(index).store(
                                    pos + self.capacity(),
                                    Ordering::Release,
                                );
                                return Ok(Some(result));
                            }
                        },
                        diff if diff < 0 => return Ok(None),
                        _ => continue,
                    }
                })
            })
        })
    }

    #[pyo3(signature = (item, timeout=None))]
    fn put(
        &self,
        item: &[u8],
        timeout: Option<f64>,
    ) -> PyResult<()> {
        Python::with_gil(|py| {
            py.allow_threads(|| {
                let start = Instant::now();
                loop {
                    match self.try_put(item) {
                        Ok(true) => return Ok(()),
                        Ok(false) => {},
                        Err(e) => return Err(e),
                    }

                    if let Some(t) = timeout {
                        if start.elapsed().as_secs_f64() > t {
                            return Err(PyBlockingIOError::new_err(
                                "Queue full",
                            ));
                        }
                    }

                    Python::with_gil(|_| {
                        std::thread::sleep(Duration::from_millis(1));
                    });
                }
            })
        })
    }

    fn put_nowait(
        &self,
        item: &[u8],
    ) -> PyResult<()> {
        if self.try_put(item)? {
            Ok(())
        } else {
            Err(QueueFull::new_err("Queue is full"))
        }
    }

    #[pyo3(signature = (timeout=None))]
    fn get(
        &self,
        timeout: Option<f64>,
    ) -> PyResult<Vec<u8>> {
        Python::with_gil(|py| {
            py.allow_threads(|| {
                let start = Instant::now();
                loop {
                    match self.try_get() {
                        Ok(Some(item)) => return Ok(item),
                        Ok(None) => {},
                        Err(e) => return Err(e),
                    }

                    if let Some(t) = timeout {
                        if start.elapsed().as_secs_f64() > t {
                            return Err(PyBlockingIOError::new_err(
                                "Queue empty",
                            ));
                        }
                    }

                    Python::with_gil(|_| {
                        std::thread::sleep(Duration::from_millis(1));
                    });
                }
            })
        })
    }

    fn get_nowait(&self) -> PyResult<Vec<u8>> {
        self.try_get()?
            .ok_or_else(|| QueueEmpty::new_err("Queue is empty"))
    }

    fn __len__(&self) -> PyResult<usize> {
        self.check_active()?;
        let head = self.dequeue_pos().load(Ordering::Acquire);
        let tail = self.enqueue_pos().load(Ordering::Acquire);
        Ok(tail.saturating_sub(head))
    }

    fn __bool__(&self) -> PyResult<bool> {
        Ok(self.__len__()? > 0)
    }

    fn full(&self) -> PyResult<bool> {
        self.check_active()?;
        Ok(self.__len__()? >= self.capacity())
    }

    fn empty(&self) -> PyResult<bool> {
        Ok(self.__len__()? == 0)
    }

    #[getter]
    fn element_size(&self) -> PyResult<usize> {
        self.check_active()?;
        Ok(self.get_element_size())
    }

    #[getter]
    fn maxsize(&self) -> PyResult<usize> {
        self.check_active()?;
        Ok(self.capacity())
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

#[pymodule]
fn rpc_queue(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Queue>()?;
    Ok(())
}
