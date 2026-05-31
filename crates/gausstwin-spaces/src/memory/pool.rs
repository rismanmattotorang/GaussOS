use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam::queue::SegQueue;
use parking_lot::RwLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryPoolError {
    #[error("Failed to allocate memory: {0}")]
    AllocationFailed(String),
    #[error("Invalid block size")]
    InvalidBlockSize,
    #[error("Pool capacity exceeded")]
    CapacityExceeded,
}

/// A high-performance, thread-safe memory pool for efficient allocation and reuse
pub struct MemoryPool<T> {
    /// Queue of free blocks
    free_blocks: SegQueue<NonNull<u8>>,
    /// Total number of allocated blocks
    allocated_blocks: AtomicUsize,
    /// Maximum number of blocks allowed
    capacity: usize,
    /// Size of each block
    block_size: usize,
    /// Block alignment
    alignment: usize,
    /// Memory layout for allocations
    layout: Layout,
    /// Phantom data for type T
    _phantom: PhantomData<T>,
    /// Statistics for monitoring
    stats: RwLock<PoolStats>,
}

#[derive(Debug, Default)]
pub struct PoolStats {
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub peak_memory_usage: usize,
    pub current_memory_usage: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

impl<T> MemoryPool<T> {
    /// Create a new memory pool with the specified capacity
    pub fn new(capacity: usize) -> Result<Self, MemoryPoolError> {
        let block_size = mem::size_of::<T>();
        let alignment = mem::align_of::<T>();

        if block_size == 0 {
            return Err(MemoryPoolError::InvalidBlockSize);
        }

        let layout = Layout::from_size_align(block_size, alignment)
            .map_err(|e| MemoryPoolError::AllocationFailed(e.to_string()))?;

        Ok(Self {
            free_blocks: SegQueue::new(),
            allocated_blocks: AtomicUsize::new(0),
            capacity,
            block_size,
            alignment,
            layout,
            _phantom: PhantomData,
            stats: RwLock::new(PoolStats::default()),
        })
    }

    /// Allocate a new block from the pool
    pub fn allocate(&self) -> Result<NonNull<u8>, MemoryPoolError> {
        // Try to reuse a block from the free list
        if let Some(block) = self.free_blocks.pop() {
            let mut stats = self.stats.write();
            stats.cache_hits += 1;
            stats.current_memory_usage += self.block_size;
            stats.peak_memory_usage = stats.peak_memory_usage.max(stats.current_memory_usage);
            return Ok(block);
        }

        // Allocate a new block if under capacity
        let current_blocks = self.allocated_blocks.load(Ordering::Relaxed);
        if current_blocks >= self.capacity {
            return Err(MemoryPoolError::CapacityExceeded);
        }

        // Allocate new memory
        let ptr = unsafe {
            let ptr = alloc(self.layout);
            NonNull::new(ptr).ok_or_else(|| {
                MemoryPoolError::AllocationFailed("Failed to allocate memory".into())
            })?
        };

        self.allocated_blocks.fetch_add(1, Ordering::Relaxed);

        let mut stats = self.stats.write();
        stats.total_allocations += 1;
        stats.cache_misses += 1;
        stats.current_memory_usage += self.block_size;
        stats.peak_memory_usage = stats.peak_memory_usage.max(stats.current_memory_usage);

        Ok(ptr)
    }

    /// Return a block to the pool
    pub fn deallocate(&self, ptr: NonNull<u8>) {
        self.free_blocks.push(ptr);
        
        let mut stats = self.stats.write();
        stats.total_deallocations += 1;
        stats.current_memory_usage -= self.block_size;
    }

    /// Get current pool statistics
    pub fn get_stats(&self) -> PoolStats {
        self.stats.read().clone()
    }
}

impl<T> Drop for MemoryPool<T> {
    fn drop(&mut self) {
        // Deallocate all blocks in the free list
        while let Some(ptr) = self.free_blocks.pop() {
            unsafe {
                dealloc(ptr.as_ptr(), self.layout);
            }
        }
    }
}

unsafe impl<T: Send> Send for MemoryPool<T> {}
unsafe impl<T: Sync> Sync for MemoryPool<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_memory_pool_basic() {
        let pool = MemoryPool::<u64>::new(10).unwrap();
        
        // Test allocation
        let ptr1 = pool.allocate().unwrap();
        let ptr2 = pool.allocate().unwrap();
        
        // Test deallocation
        pool.deallocate(ptr1);
        pool.deallocate(ptr2);
        
        let stats = pool.get_stats();
        assert_eq!(stats.total_allocations, 2);
        assert_eq!(stats.total_deallocations, 2);
    }

    #[test]
    fn test_memory_pool_capacity() {
        let pool = MemoryPool::<u64>::new(2).unwrap();
        
        let ptr1 = pool.allocate().unwrap();
        let ptr2 = pool.allocate().unwrap();
        
        // Should fail due to capacity
        assert!(pool.allocate().is_err());
        
        pool.deallocate(ptr1);
        pool.deallocate(ptr2);
    }

    #[test]
    fn test_memory_pool_threaded() {
        let pool = Arc::new(MemoryPool::<u64>::new(1000).unwrap());
        let mut handles = vec![];

        for _ in 0..10 {
            let pool = Arc::clone(&pool);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let ptr = pool.allocate().unwrap();
                    pool.deallocate(ptr);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = pool.get_stats();
        assert_eq!(stats.total_allocations, 1000);
        assert_eq!(stats.total_deallocations, 1000);
    }
} 