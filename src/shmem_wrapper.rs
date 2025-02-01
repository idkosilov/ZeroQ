use shared_memory::Shmem;

/// A wrapper around `Shmem` to safely enable `Send` and `Sync` traits,
/// allowing shared memory to be safely used across threads.
pub struct ShmemWrapper {
    shmem: Shmem,
}

// Manually implementing `Send` and `Sync` because `Shmem` is not marked as such by default.
// This is safe as long as proper synchronization mechanisms are used when accessing shared memory.
unsafe impl Send for ShmemWrapper {}
unsafe impl Sync for ShmemWrapper {}

impl ShmemWrapper {
    /// Creates a new `ShmemWrapper` from an existing `Shmem` instance.
    pub fn new(shmem: Shmem) -> Self {
        Self { shmem }
    }

    /// Returns a raw pointer to the beginning of the shared memory region.
    pub fn as_ptr(&self) -> *const u8 {
        self.shmem.as_ptr()
    }

    /// Returns the total size of the shared memory region in bytes.
    pub fn len(&self) -> usize {
        self.shmem.len()
    }
}
