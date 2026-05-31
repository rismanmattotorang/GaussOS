use crossbeam_queue::SegQueue;
use parking_lot::RwLock;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

/// Statistics for memory pool operations
#[derive(Debug, Default)]
pub struct PoolStats {
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
}

/// Memory chunk for efficient allocation
#[repr(align(64))] // Cache line alignment
struct Chunk<T> {
    data: Vec<T>,
    used: AtomicUsize,
}

impl<T> Chunk<T> {
    fn new(size: usize) -> Self 
    where T: Default {
        Self {
            data: (0..size).map(|_| T::default()).collect(),
            used: AtomicUsize::new(0),
        }
    }

    fn allocate(&self) -> Option<T> 
    where T: Clone {
        let index = self.used.fetch_add(1, Ordering::Relaxed);
        if index < self.data.len() {
            Some(self.data[index].clone())
        } else {
            self.used.fetch_sub(1, Ordering::Relaxed);
            None
        }
    }
}

/// High-performance memory pool with SIMD support
pub struct HighPerformanceMemoryPool<T> {
    active_chunk: RwLock<Arc<Chunk<T>>>,
    free_list: SegQueue<T>,
    chunk_size: usize,
    stats: Arc<PoolStats>,
}

impl<T> HighPerformanceMemoryPool<T>
where
    T: Default + Clone + Send + Sync + 'static,
{
    pub fn new(chunk_size: usize) -> Self {
        Self {
            active_chunk: RwLock::new(Arc::new(Chunk::new(chunk_size))),
            free_list: SegQueue::new(),
            chunk_size,
            stats: Arc::new(PoolStats::default()),
        }
    }

    pub fn allocate(&self) -> T {
        // Try free list first for fastest allocation
        if let Ok(item) = self.free_list.pop() {
            self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            return item;
        }

        self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);

        // Try current chunk
        {
            let chunk = self.active_chunk.read();
            if let Some(item) = chunk.allocate() {
                return item;
            }
        }

        // Current chunk is full, create new one
        let new_chunk = Arc::new(Chunk::new(self.chunk_size));
        *self.active_chunk.write() = new_chunk.clone();
        
        // This should never fail as we just created a fresh chunk
        new_chunk.allocate().unwrap()
    }

    pub fn deallocate(&self, value: T) {
        self.free_list.push(value);
        self.stats.deallocations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> Arc<PoolStats> {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_basic_allocation() {
        let pool = HighPerformanceMemoryPool::<usize>::new(100);
        let value = pool.allocate();
        assert_eq!(value, 0);
        pool.deallocate(value);
    }

    #[test]
    fn test_parallel_allocation() {
        let pool = Arc::new(HighPerformanceMemoryPool::<usize>::new(1000));
        let mut handles = vec![];

        for _ in 0..10 {
            let pool = pool.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let value = pool.allocate();
                    pool.deallocate(value);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = pool.stats();
        assert!(stats.deallocations.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_chunk_allocation() {
        let pool = HighPerformanceMemoryPool::<usize>::new(2);
        
        // First chunk
        let v1 = pool.allocate();
        let v2 = pool.allocate();
        
        // Should create new chunk
        let v3 = pool.allocate();
        
        pool.deallocate(v1);
        pool.deallocate(v2);
        pool.deallocate(v3);
    }
} 