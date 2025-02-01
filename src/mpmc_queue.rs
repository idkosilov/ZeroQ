use std::marker::PhantomData;
use std::mem::{align_of, size_of, MaybeUninit};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn compute_required_size(
    element_size: usize,
    capacity: usize,
) -> usize {
    use std::mem::{align_of, size_of};

    let header_size = size_of::<MpmcQueueHeader>();

    let cells_offset = align_up(header_size, align_of::<Cell>());
    let cells_size = capacity * size_of::<Cell>();

    let data_offset = align_up(cells_offset + cells_size, align_of::<u8>());
    let data_size = capacity * element_size;
    data_offset + data_size
}

/// Errors that can occur when using the MPMC queue.
#[derive(Debug)]
pub enum MpmcQueueError {
    /// The provided source slice length does not match the expected element size.
    InvalidSourceLength { expected: usize, actual: usize },
    /// The provided destination slice length does not match the expected element size.
    InvalidDestinationLength { expected: usize, actual: usize },
    /// The queue is full and cannot accept more elements.
    QueueFull,
    /// The queue is empty and there is no element to dequeue.
    QueueEmpty,
    /// The provided buffer is too small to hold the queue structure.
    BufferTooSmall { required: usize, provided: usize },
    /// The provided buffer does not have the required alignment.
    BufferMisaligned { expected: usize, actual: usize },
    /// The provided buffer size is not a power of two.
    BufferSizeNotPowerOfTwo { actual: usize },
}

/// The header for the MPMC queue stored on a pre-allocated buffer.
///
/// This header is stored at the beginning of the buffer.
#[repr(C)]
pub struct MpmcQueueHeader {
    /// The size (in bytes) of a single element.
    pub element_size: usize,
    /// A bitmask used for indexing into the circular buffer. (buffer_size - 1)
    pub buffer_mask: usize,
    /// The current enqueue position (atomic).
    pub enqueue_pos: AtomicUsize,
    /// The current dequeue position (atomic).
    pub dequeue_pos: AtomicUsize,
}

/// A cell in the queue that holds sequence metadata.
///
/// Each cell corresponds to a slot in the queue and is used to determine
/// whether the slot is ready for an enqueue or dequeue operation.
#[repr(C)]
struct Cell {
    /// The sequence number used for synchronizing enqueue/dequeue operations.
    sequence: AtomicUsize,
}

/// Aligns `offset` upwards to the nearest multiple of `align`.
///
/// # Parameters
///
/// - `offset`: The original offset value.
/// - `align`: The alignment to align to (must be a power of two).
///
/// # Returns
///
/// The smallest value greater than or equal to `offset` that is a multiple of `align`.
#[inline]
fn align_up(
    offset: usize,
    align: usize,
) -> usize {
    (offset + align - 1) & !(align - 1)
}

/// A bounded multiple-producer, multiple-consumer (MPMC) queue stored entirely in a
/// pre-allocated buffer.
///
/// The queue supports concurrent enqueues and dequeues by multiple threads without
/// locks, using atomic operations.
pub struct MpmcQueueOnBuffer<'a> {
    /// The base pointer to the start of the allocated buffer.
    base: NonNull<u8>,
    /// Phantom data to associate the lifetime of the queue with the lifetime of the buffer.
    _marker: PhantomData<&'a mut [MaybeUninit<u8>]>,
}

unsafe impl<'a> Send for MpmcQueueOnBuffer<'a> {}
unsafe impl<'a> Sync for MpmcQueueOnBuffer<'a> {}

impl<'a> MpmcQueueOnBuffer<'a> {
    /// Validates the provided queue parameters and computes the memory layout offsets.
    ///
    /// This function checks that:
    /// - `buffer_size` is at least 2 and is a power of two.
    /// - The buffer is large enough to store the header, cells array, and data region.
    ///
    /// # Parameters
    ///
    /// - `buffer`: The buffer where the queue will be stored.
    /// - `element_size`: Size (in bytes) of a single element.
    /// - `buffer_size`: The number of slots in the queue.
    ///
    /// # Returns
    ///
    /// On success, returns a tuple containing:
    /// - `header_size`: Size of the queue header.
    /// - `cells_offset`: Offset from the start of the buffer to the cells array.
    /// - `data_offset`: Offset from the start of the buffer to the data region.
    /// - `required_size`: Total required buffer size.
    ///
    /// On failure, returns an appropriate `MpmcQueueError`.
    fn validate_and_compute_layout(
        buffer: &[MaybeUninit<u8>],
        element_size: usize,
        buffer_size: usize,
    ) -> Result<(usize, usize, usize, usize), MpmcQueueError> {
        if buffer_size < 2 {
            return Err(MpmcQueueError::BufferTooSmall {
                required: 2,
                provided: buffer_size,
            });
        }
        if !buffer_size.is_power_of_two() {
            return Err(MpmcQueueError::BufferSizeNotPowerOfTwo {
                actual: buffer_size,
            });
        }

        let header_size = size_of::<MpmcQueueHeader>();
        let cells_offset = align_up(header_size, align_of::<Cell>());
        let cells_size = buffer_size * size_of::<Cell>();

        let data_offset = align_up(cells_offset + cells_size, align_of::<u8>());
        let data_size = buffer_size * element_size;

        let required_size = data_offset + data_size;
        if buffer.len() < required_size {
            return Err(MpmcQueueError::BufferTooSmall {
                required: required_size,
                provided: buffer.len(),
            });
        }

        Ok((header_size, cells_offset, data_offset, required_size))
    }

    /// Initializes the MPMC queue within a pre-allocated buffer.
    ///
    /// This function sets up the queue header, cell metadata, and reserves space for data.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `buffer` is valid for writes of `required_size` bytes.
    /// - `buffer` is properly aligned for `MpmcQueueHeader`.
    ///
    /// # Parameters
    ///
    /// - `buffer`: A mutable slice of `MaybeUninit<u8>` where the queue will be stored.
    /// - `element_size`: The size (in bytes) of each element.
    /// - `buffer_size`: The number of slots in the queue (must be ≥2 and a power of two).
    ///
    /// # Returns
    ///
    /// Returns an instance of `MpmcQueueOnBuffer` on success, or an appropriate error.
    pub unsafe fn init_on_buffer(
        buffer: &'a mut [MaybeUninit<u8>],
        element_size: usize,
        buffer_size: usize,
        new: bool,
    ) -> Result<Self, MpmcQueueError> {
        let (_header_size, cells_offset, _data_offset, _required_size) =
            Self::validate_and_compute_layout(
                buffer,
                element_size,
                buffer_size,
            )?;

        let buffer_ptr = buffer.as_mut_ptr() as *mut u8;
        let header_align = align_of::<MpmcQueueHeader>();

        if (buffer_ptr as usize) % header_align != 0 {
            return Err(MpmcQueueError::BufferMisaligned {
                expected: header_align,
                actual: buffer_ptr as usize % header_align,
            });
        }

        if new {
            // Initialize the queue header and cells array in the buffer.
            Self::init_header(buffer_ptr, element_size, buffer_size);
            Self::init_cells(
                buffer_ptr.add(cells_offset) as *mut Cell,
                buffer_size,
            );
        }

        Ok(Self {
            base: NonNull::new_unchecked(buffer_ptr),
            _marker: PhantomData,
        })
    }

    /// Initializes the queue header at the given pointer.
    ///
    /// The header is written to the beginning of the buffer and contains the element size,
    /// a buffer mask (buffer_size - 1), and atomic positions for enqueue and dequeue.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `header_ptr` is valid for writes of a `MpmcQueueHeader`.
    #[inline]
    unsafe fn init_header(
        header_ptr: *mut u8,
        element_size: usize,
        buffer_size: usize,
    ) {
        std::ptr::write(
            header_ptr as *mut MpmcQueueHeader,
            MpmcQueueHeader {
                element_size,
                buffer_mask: buffer_size - 1,
                enqueue_pos: AtomicUsize::new(0),
                dequeue_pos: AtomicUsize::new(0),
            },
        );
    }

    /// Initializes the array of queue cells at the given pointer.
    ///
    /// Each cell is initialized with a sequence number equal to its index.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `cells_ptr` points to a valid block of memory
    /// that can hold `buffer_size` cells.
    #[inline]
    unsafe fn init_cells(
        cells_ptr: *mut Cell,
        buffer_size: usize,
    ) {
        for i in 0..buffer_size {
            std::ptr::write(
                cells_ptr.add(i),
                Cell { sequence: AtomicUsize::new(i) },
            );
        }
    }

    /// Returns a reference to the queue header.
    pub fn header(&self) -> &MpmcQueueHeader {
        unsafe { &*(self.base.as_ptr() as *const MpmcQueueHeader) }
    }

    /// Returns a mutable pointer to the start of the cells array.
    ///
    /// The cells array contains metadata for each slot in the queue.
    #[inline]
    fn cells_ptr(&self) -> *mut Cell {
        let header_size = size_of::<MpmcQueueHeader>();
        let cells_offset = align_up(header_size, align_of::<Cell>());
        unsafe { self.base.as_ptr().add(cells_offset) as *mut Cell }
    }

    /// Returns a mutable pointer to the data region.
    ///
    /// The data region is where the actual enqueued elements are stored.
    #[inline]
    fn data_ptr(&self) -> *mut u8 {
        let header_size = size_of::<MpmcQueueHeader>();
        let cells_offset = align_up(header_size, align_of::<Cell>());
        let buffer_size = self.header().buffer_mask + 1;
        let cells_size = buffer_size * size_of::<Cell>();
        let data_offset = align_up(cells_offset + cells_size, align_of::<u8>());
        unsafe { self.base.as_ptr().add(data_offset) }
    }

    /// Computes the index into the circular buffer for a given position.
    ///
    /// # Parameters
    ///
    /// - `pos`: The absolute position (either enqueue or dequeue).
    ///
    /// # Returns
    ///
    /// The index within the cells and data array corresponding to `pos`.
    #[inline]
    fn cell_index(
        &self,
        pos: usize,
    ) -> usize {
        pos & self.header().buffer_mask
    }

    /// Validates that the source slice used for enqueue has the correct length.
    ///
    /// # Parameters
    ///
    /// - `src`: The source byte slice to be enqueued.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the length matches the expected element size, or an error otherwise.
    #[inline]
    fn validate_enqueue_src(
        &self,
        src: &[u8],
    ) -> Result<(), MpmcQueueError> {
        let header = self.header();
        if src.len() != header.element_size {
            Err(MpmcQueueError::InvalidSourceLength {
                expected: header.element_size,
                actual: src.len(),
            })
        } else {
            Ok(())
        }
    }

    /// Validates that the destination slice used for dequeue has the correct length.
    ///
    /// # Parameters
    ///
    /// - `dst`: The destination byte slice where the dequeued element will be stored.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the length matches the expected element size, or an error otherwise.
    #[inline]
    fn validate_dequeue_dst(
        &self,
        dst: &[u8],
    ) -> Result<(), MpmcQueueError> {
        let header = self.header();
        if dst.len() != header.element_size {
            Err(MpmcQueueError::InvalidDestinationLength {
                expected: header.element_size,
                actual: dst.len(),
            })
        } else {
            Ok(())
        }
    }

    /// Attempts to reserve an enqueue slot in the queue.
    ///
    /// This function uses an atomic compare-and-swap loop to reserve a slot if available.
    ///
    /// # Returns
    ///
    /// On success, returns `Some(position)` representing the reserved enqueue position.
    /// Returns `None` if the queue is full.
    fn try_reserve_enqueue_slot(&self) -> Option<usize> {
        let header = self.header();
        let buffer_mask = header.buffer_mask;
        let mut pos = header.enqueue_pos.load(Ordering::Relaxed);
        loop {
            let index = pos & buffer_mask;
            let cell = self.cell(index);
            let seq = cell.sequence.load(Ordering::Acquire);
            let dif = seq as isize - pos as isize;
            if dif == 0 {
                match header.enqueue_pos.compare_exchange_weak(
                    pos,
                    pos + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return Some(pos),
                    Err(new_pos) => pos = new_pos,
                }
            } else if dif < 0 {
                return None;
            } else {
                pos = header.enqueue_pos.load(Ordering::Relaxed);
            }
        }
    }

    /// Writes an element into the reserved enqueue slot.
    ///
    /// This function copies the source data into the data region and then updates
    /// the slot's sequence to indicate that the element is available for dequeue.
    ///
    /// # Parameters
    ///
    /// - `pos`: The reserved enqueue position.
    /// - `src`: The source byte slice to be enqueued.
    #[inline]
    fn write_slot(
        &self,
        pos: usize,
        src: &[u8],
    ) {
        let header = self.header();
        let index = self.cell_index(pos);
        let data_offset = index * header.element_size;
        let dst = unsafe { self.data_ptr().add(data_offset) };
        unsafe {
            std::ptr::copy_nonoverlapping(
                src.as_ptr(),
                dst,
                header.element_size,
            );
        }
        // Mark the cell as available for dequeue by updating its sequence.
        unsafe {
            self.cells_ptr()
                .add(index)
                .as_mut()
                .unwrap()
                .sequence
                .store(pos + 1, Ordering::Release);
        }
    }

    /// Attempts to reserve a dequeue slot from the queue.
    ///
    /// This function uses an atomic compare-and-swap loop to reserve a slot that is ready
    /// for dequeue.
    ///
    /// # Returns
    ///
    /// On success, returns `Some(position)` representing the reserved dequeue position.
    /// Returns `None` if the queue is empty.
    fn try_reserve_dequeue_slot(&self) -> Option<usize> {
        let header = self.header();
        let buffer_mask = header.buffer_mask;
        let mut pos = header.dequeue_pos.load(Ordering::Relaxed);
        loop {
            let index = pos & buffer_mask;
            let cell = self.cell(index);
            let seq = cell.sequence.load(Ordering::Acquire);
            let dif = seq as isize - (pos as isize + 1);
            if dif == 0 {
                match header.dequeue_pos.compare_exchange_weak(
                    pos,
                    pos + 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return Some(pos),
                    Err(new_pos) => pos = new_pos,
                }
            } else if dif < 0 {
                return None;
            } else {
                pos = header.dequeue_pos.load(Ordering::Relaxed);
            }
        }
    }

    /// Reads an element from the reserved dequeue slot into the provided destination slice.
    ///
    /// This function copies the data from the queue's data region into `dst` and then updates
    /// the slot's sequence to indicate that it is available for enqueue.
    ///
    /// # Parameters
    ///
    /// - `pos`: The reserved dequeue position.
    /// - `dst`: The destination byte slice where the dequeued element will be stored.
    #[inline]
    fn read_slot(
        &self,
        pos: usize,
        dst: &mut [u8],
    ) {
        let header = self.header();
        let index = self.cell_index(pos);
        let data_offset = index * header.element_size;
        let src = unsafe { self.data_ptr().add(data_offset) };
        unsafe {
            std::ptr::copy_nonoverlapping(
                src,
                dst.as_mut_ptr(),
                header.element_size,
            );
        }
        // Mark the cell as free for the next enqueue by updating its sequence.
        unsafe {
            self.cells_ptr()
                .add(index)
                .as_mut()
                .unwrap()
                .sequence
                .store(pos + header.buffer_mask + 1, Ordering::Release);
        }
    }

    /// Enqueues an element into the queue.
    ///
    /// This function validates the source slice length, attempts to reserve an enqueue slot,
    /// and writes the element into the queue.
    ///
    /// # Parameters
    ///
    /// - `src`: The source byte slice containing the element to enqueue.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an appropriate `MpmcQueueError` if the queue is full or
    /// if the source length is invalid.
    pub fn enqueue(
        &self,
        src: &[u8],
    ) -> Result<(), MpmcQueueError> {
        self.validate_enqueue_src(src)?;
        if let Some(pos) = self.try_reserve_enqueue_slot() {
            self.write_slot(pos, src);
            Ok(())
        } else {
            Err(MpmcQueueError::QueueFull)
        }
    }

    /// Dequeues an element from the queue.
    ///
    /// This function validates the destination slice length, attempts to reserve a dequeue slot,
    /// and copies the dequeued element into `dst`.
    ///
    /// # Parameters
    ///
    /// - `dst`: The destination byte slice where the dequeued element will be stored.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an appropriate `MpmcQueueError` if the queue is empty or
    /// if the destination length is invalid.
    pub fn dequeue(
        &self,
        dst: &mut [u8],
    ) -> Result<(), MpmcQueueError> {
        self.validate_dequeue_dst(dst)?;
        if let Some(pos) = self.try_reserve_dequeue_slot() {
            self.read_slot(pos, dst);
            Ok(())
        } else {
            Err(MpmcQueueError::QueueEmpty)
        }
    }

    /// Retrieves a reference to the cell at the specified index.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `index` is within the bounds of the cells array.
    #[inline]
    fn cell(
        &self,
        index: usize,
    ) -> &Cell {
        unsafe { &*self.cells_ptr().add(index) }
    }
}
