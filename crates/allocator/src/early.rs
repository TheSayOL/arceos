use super::{AllocError, AllocResult};
use core::alloc::Layout;
use core::ptr::NonNull;

/// EarlyAllocator
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    byte_ptr: usize,
    byte_alloc_count: usize,
    page_ptr: usize,
    start: usize,
    size: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    /// Creates an empty [`EarlyAllocator`].
    pub const fn new() -> Self {
        Self {
            byte_ptr: 0,
            byte_alloc_count: 0,
            page_ptr: 0,
            start: 0,
            size: 0,
        }
    }
    /// Initializes the allocator with the given region.
    pub fn init(&mut self, start_vaddr: usize, size: usize) {
        self.byte_ptr = start_vaddr;
        self.page_ptr = start_vaddr + size;
        self.byte_alloc_count = 0;
        self.start = start_vaddr;
        self.size = size;
    }
    /// Allocate arbitrary number of bytes. Returns the left bound of the
    /// allocated region.
    pub fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let size = layout.size();
        if self.available_bytes() < size {
            return Err(AllocError::NoMemory);
        }
        self.byte_alloc_count += 1;
        let ret = Ok(NonNull::new(self.byte_ptr as *mut u8).unwrap());
        self.byte_ptr += size;
        ret
    }
    /// Gives back the allocated region to the byte allocator.
    pub fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        self.byte_alloc_count -= 1;
        if self.byte_alloc_count == 0 {
            self.byte_ptr = self.start;
        }
    }
    /// Allocates contiguous pages.
    pub fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        if align_pow2 % PAGE_SIZE != 0 {
            return Err(AllocError::InvalidParam);
        }
        if !(align_pow2 / PAGE_SIZE).is_power_of_two() {
            return Err(AllocError::InvalidParam);
        }
        let base = self.page_ptr - num_pages * PAGE_SIZE - self.page_ptr % align_pow2;
        if base < self.byte_ptr {
            return Err(AllocError::NoMemory);
        }
        self.page_ptr = base;
        Ok(base)
    }
    /// Gives back the allocated pages starts from `pos` to the page allocator.
    pub fn dealloc_pages(&self, _pos: usize, _num_pages: usize) {
        // do nothing
    }
    /// Returns the number of allocated bytes in the byte allocator.
    pub fn used_bytes(&self) -> usize {
        self.byte_ptr - self.start
    }
    /// Returns the number of available bytes in the byte allocator.
    pub fn available_bytes(&self) -> usize {
        self.page_ptr - self.byte_ptr
    }
    /// Returns the number of allocated pages in the page allocator.
    pub fn used_pages(&self) -> usize {
        (self.start + self.size - self.page_ptr) / PAGE_SIZE
    }
    /// Returns the number of available pages in the page allocator.
    pub fn available_pages(&self) -> usize {
        (self.page_ptr - self.byte_ptr) / PAGE_SIZE
    }
}
