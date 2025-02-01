use shared_memory::Shmem;

pub struct ShmemWrapper {
    shmem: Shmem,
}

unsafe impl Send for ShmemWrapper {}

unsafe impl Sync for ShmemWrapper {}

impl ShmemWrapper {
    pub fn new(shmem: Shmem) -> Self {
        Self { shmem }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.shmem.as_ptr()
    }

    pub fn len(&self) -> usize {
        self.shmem.len()
    }
}

impl Drop for ShmemWrapper {
    fn drop(&mut self) {
        self.shmem.set_owner(false);
    }
}
