use std::marker::PhantomData;
use std::mem::{align_of, size_of, MaybeUninit};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Computes the required buffer size for an `MpmcQueueOnBuffer`
/// given the `element_size` and `capacity`.
pub fn compute_required_size(element_size: usize, capacity: usize) -> usize {
    use std::mem::{align_of, size_of};

    let header_size = size_of::<MpmcQueueHeader>();

    let cells_offset = align_up(header_size, align_of::<Cell>());
    let cells_size = capacity * size_of::<Cell>();

    let data_offset = align_up(cells_offset + cells_size, align_of::<u8>());
    let data_size = capacity * element_size;
    data_offset + data_size
}

/// Errors that can occur when using `MpmcQueueOnBuffer`.
#[derive(Debug)]
pub enum MpmcQueueError {
    InvalidSourceLength { expected: usize, actual: usize },
    InvalidDestinationLength { expected: usize, actual: usize },
    QueueFull,
    QueueEmpty,
    BufferTooSmall { required: usize, provided: usize },
    BufferMisaligned { expected: usize, actual: usize },
    BufferSizeNotPowerOfTwo { actual: usize },
}

/// Header structure stored at the beginning of the queue buffer.
/// Contains metadata required for queue operation.
#[repr(C)]
pub struct MpmcQueueHeader {
    pub element_size: usize,
    pub buffer_mask: usize,
    pub enqueue_pos: AtomicUsize,
    pub dequeue_pos: AtomicUsize,
}

/// Metadata structure for each queue slot.
/// Holds a sequence number used for synchronization.
#[repr(C)]
struct Cell {
    sequence: AtomicUsize,
}

/// Aligns an offset upwards to the nearest multiple of `align`.
#[inline]
fn align_up(offset: usize, align: usize) -> usize {
    (offset + align - 1) & !(align - 1)
}

/// Lock-free MPMC queue stored in a pre-allocated buffer.
///
/// This implementation uses atomic operations for synchronization
/// and does not require explicit locking.
pub struct MpmcQueueOnBuffer<'a> {
    base: NonNull<u8>,
    _marker: PhantomData<&'a mut [MaybeUninit<u8>]>,
}

unsafe impl Send for MpmcQueueOnBuffer<'_> {}
unsafe impl Sync for MpmcQueueOnBuffer<'_> {}

impl<'a> MpmcQueueOnBuffer<'a> {
    /// Validates buffer layout and ensures it meets required conditions.
    /// Returns offsets and sizes for different queue components.
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

    /// Initializes the queue in a pre-allocated buffer.
    ///
    /// # Safety
    /// The caller must ensure that the buffer is properly aligned
    /// and has sufficient size to hold the queue.
    pub unsafe fn init_on_buffer(
        buffer: &'a mut [MaybeUninit<u8>],
        element_size: usize,
        buffer_size: usize,
        new: bool,
    ) -> Result<Self, MpmcQueueError> {
        let (_header_size, cells_offset, _data_offset, _required_size) =
            Self::validate_and_compute_layout(buffer, element_size, buffer_size)?;

        let buffer_ptr = buffer.as_mut_ptr() as *mut u8;
        let header_align = align_of::<MpmcQueueHeader>();

        if (buffer_ptr as usize) % header_align != 0 {
            return Err(MpmcQueueError::BufferMisaligned {
                expected: header_align,
                actual: buffer_ptr as usize % header_align,
            });
        }

        if new {
            Self::init_header(buffer_ptr, element_size, buffer_size);
            Self::init_cells(buffer_ptr.add(cells_offset) as *mut Cell, buffer_size);
        }

        Ok(Self {
            base: NonNull::new_unchecked(buffer_ptr),
            _marker: PhantomData,
        })
    }

    /// Initializes the queue header at the given pointer.
    #[inline]
    unsafe fn init_header(header_ptr: *mut u8, element_size: usize, buffer_size: usize) {
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

    /// Initializes the sequence numbers for each cell.
    #[inline]
    unsafe fn init_cells(cells_ptr: *mut Cell, buffer_size: usize) {
        for i in 0..buffer_size {
            std::ptr::write(
                cells_ptr.add(i),
                Cell {
                    sequence: AtomicUsize::new(i),
                },
            );
        }
    }

    /// Retrieves a reference to the queue header.
    pub fn header(&self) -> &MpmcQueueHeader {
        unsafe { &*(self.base.as_ptr() as *const MpmcQueueHeader) }
    }

    #[inline]
    fn cells_ptr(&self) -> *mut Cell {
        let header_size = size_of::<MpmcQueueHeader>();
        let cells_offset = align_up(header_size, align_of::<Cell>());
        unsafe { self.base.as_ptr().add(cells_offset) as *mut Cell }
    }

    #[inline]
    fn data_ptr(&self) -> *mut u8 {
        let header_size = size_of::<MpmcQueueHeader>();
        let cells_offset = align_up(header_size, align_of::<Cell>());
        let buffer_size = self.header().buffer_mask + 1;
        let cells_size = buffer_size * size_of::<Cell>();
        let data_offset = align_up(cells_offset + cells_size, align_of::<u8>());
        unsafe { self.base.as_ptr().add(data_offset) }
    }

    #[inline]
    fn cell_index(&self, pos: usize) -> usize {
        pos & self.header().buffer_mask
    }

    #[inline]
    fn validate_enqueue_src(&self, src: &[u8]) -> Result<(), MpmcQueueError> {
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

    #[inline]
    fn validate_dequeue_dst(&self, dst: &[u8]) -> Result<(), MpmcQueueError> {
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

    /// Attempts to reserve a slot for enqueuing an element.
    /// Returns `Some(position)` if successful, `None` if the queue is full.
    fn try_reserve_enqueue_slot(&self) -> Option<usize> {
        let header = self.header();
        let buffer_mask = header.buffer_mask;
        let mut pos = header.enqueue_pos.load(Ordering::Relaxed);
        loop {
            let index = pos & buffer_mask;
            let cell = self.cell(index);
            let seq = cell.sequence.load(Ordering::Acquire);
            let dif = seq as isize - pos as isize;
            match dif.cmp(&0) {
                std::cmp::Ordering::Equal => {
                    match header.enqueue_pos.compare_exchange_weak(
                        pos,
                        pos + 1,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => return Some(pos),
                        Err(new_pos) => pos = new_pos,
                    }
                }
                std::cmp::Ordering::Less => return None,
                std::cmp::Ordering::Greater => {
                    pos = header.enqueue_pos.load(Ordering::Relaxed);
                }
            }
        }
    }

    #[inline]
    fn write_slot(&self, pos: usize, src: &[u8]) {
        let header = self.header();
        let index = self.cell_index(pos);
        let data_offset = index * header.element_size;
        let dst = unsafe { self.data_ptr().add(data_offset) };
        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), dst, header.element_size);
            std::sync::atomic::compiler_fence(Ordering::Release);
            self.cells_ptr()
                .add(index)
                .as_mut()
                .unwrap()
                .sequence
                .store(pos + 1, Ordering::Release);
        }
    }

    fn try_reserve_dequeue_slot(&self) -> Option<usize> {
        let header = self.header();
        let buffer_mask = header.buffer_mask;
        let mut pos = header.dequeue_pos.load(Ordering::Relaxed);
        loop {
            let index = pos & buffer_mask;
            let cell = self.cell(index);
            let seq = cell.sequence.load(Ordering::Acquire);
            let dif = seq as isize - (pos as isize + 1);
            match dif.cmp(&0) {
                std::cmp::Ordering::Equal => {
                    match header.dequeue_pos.compare_exchange_weak(
                        pos,
                        pos + 1,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => return Some(pos),
                        Err(new_pos) => pos = new_pos,
                    }
                }
                std::cmp::Ordering::Less => return None,
                std::cmp::Ordering::Greater => {
                    pos = header.dequeue_pos.load(Ordering::Relaxed);
                }
            }
        }
    }

    #[inline]
    fn read_slot(&self, pos: usize, dst: &mut [u8]) {
        let header = self.header();
        let index = self.cell_index(pos);
        let data_offset = index * header.element_size;
        let src = unsafe { self.data_ptr().add(data_offset) };
        unsafe {
            std::ptr::copy_nonoverlapping(src, dst.as_mut_ptr(), header.element_size);
        }
        unsafe {
            self.cells_ptr()
                .add(index)
                .as_mut()
                .unwrap()
                .sequence
                .store(pos + header.buffer_mask + 1, Ordering::Release);
        }
    }

    /// Attempts to enqueue an element into the queue.
    /// Returns `Ok(())` if successful, or `QueueFull` if the queue is full.
    pub fn enqueue(&self, src: &[u8]) -> Result<(), MpmcQueueError> {
        self.validate_enqueue_src(src)?;
        if let Some(pos) = self.try_reserve_enqueue_slot() {
            self.write_slot(pos, src);
            Ok(())
        } else {
            Err(MpmcQueueError::QueueFull)
        }
    }

    /// Attempts to dequeue an element from the queue.
    /// Returns `Ok(())` if successful, or `QueueEmpty` if the queue is empty.
    pub fn dequeue(&self, dst: &mut [u8]) -> Result<(), MpmcQueueError> {
        self.validate_dequeue_dst(dst)?;
        if let Some(pos) = self.try_reserve_dequeue_slot() {
            self.read_slot(pos, dst);
            Ok(())
        } else {
            Err(MpmcQueueError::QueueEmpty)
        }
    }

    /// Retrieves a reference to a queue cell at the given index.
    #[inline]
    fn cell(&self, index: usize) -> &Cell {
        unsafe { &*self.cells_ptr().add(index) }
    }
}
