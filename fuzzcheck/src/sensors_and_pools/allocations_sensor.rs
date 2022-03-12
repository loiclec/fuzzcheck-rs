use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::{SaveToStatsFolder, Sensor};

// ==== SENSOR ========

#[derive(Default)]
pub struct AllocationSensor {
    start_allocs: AllocationsStats,
    end_allocs: AllocationsStats,
}

impl SaveToStatsFolder for AllocationSensor {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        vec![]
    }
}
impl Sensor for AllocationSensor {
    type Observations = (u64, u64);

    #[no_coverage]
    fn start_recording(&mut self) {
        self.start_allocs = get_allocation_stats();
    }

    #[no_coverage]
    fn stop_recording(&mut self) {
        self.end_allocs = get_allocation_stats();
    }

    #[no_coverage]
    fn get_observations(&mut self) -> Self::Observations {
        let blocks = self.end_allocs.total_blocks - self.start_allocs.total_blocks;
        let bytes = self.end_allocs.total_bytes - self.start_allocs.total_bytes;

        (blocks, bytes)
    }
}

// ===== ALLOCATOR =====

static mut ALLOC_STATS: InternalAllocationStats = InternalAllocationStats::new();

#[derive(Default)]
struct InternalAllocationStats {
    /// Total number of allocated blocks. Does not decrease after a deallocation.
    total_blocks: AtomicU64,
    /// Total amount of allocated bytes. Does not decrease after a deallocation.
    total_bytes: AtomicU64,

    /// Number of allocated blocks currently. Decreases after a deallocation.
    curr_blocks: AtomicUsize,
    /// Amount of allocated bytes currently. Decreases after a deallocation.
    curr_bytes: AtomicUsize,
}
impl InternalAllocationStats {
    #[no_coverage]
    const fn new() -> Self {
        Self {
            total_blocks: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            curr_blocks: AtomicUsize::new(0),
            curr_bytes: AtomicUsize::new(0),
        }
    }
}

impl InternalAllocationStats {
    #[no_coverage]
    fn realloc(&mut self, size: usize, shrink: bool, delta: usize) {
        self.total_blocks.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(size as u64, Ordering::Relaxed);
        if shrink {
            self.curr_bytes.fetch_sub(delta, Ordering::Relaxed);
        } else {
            self.curr_bytes.fetch_add(delta, Ordering::Relaxed);
        }
    }
    #[no_coverage]
    fn alloc(&mut self, size: usize) {
        self.total_blocks.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(size as u64, Ordering::Relaxed);

        self.curr_blocks.fetch_add(1, Ordering::Relaxed);
        self.curr_bytes.fetch_add(size, Ordering::Relaxed);
    }

    #[no_coverage]
    fn dealloc(&mut self, size: usize) {
        self.curr_blocks.fetch_sub(1, Ordering::Relaxed);
        self.curr_bytes.fetch_sub(size, Ordering::Relaxed);
    }
}

#[derive(Debug)]
pub struct CountingAllocator<A>(pub A)
where
    A: GlobalAlloc;

unsafe impl<A> GlobalAlloc for CountingAllocator<A>
where
    A: GlobalAlloc,
{
    #[no_coverage]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.0.alloc(layout);
        if ptr.is_null() {
            return ptr;
        }
        let size = layout.size();
        ALLOC_STATS.alloc(size);
        ptr
    }

    #[no_coverage]
    unsafe fn realloc(&self, old_ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = self.0.realloc(old_ptr, layout, new_size);
        if new_ptr.is_null() {
            return new_ptr;
        }
        let old_size = layout.size();
        let (shrink, delta) = if new_size < old_size {
            (true, old_size - new_size)
        } else {
            (false, new_size - old_size)
        };
        ALLOC_STATS.realloc(new_size, shrink, delta);
        new_ptr
    }
    #[no_coverage]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.dealloc(ptr, layout);

        let size = layout.size();
        ALLOC_STATS.dealloc(size);
    }
}

#[derive(Default)]
struct AllocationsStats {
    /// Total number of allocated blocks. Does not decrease after a deallocation.
    total_blocks: u64,
    /// Total number of allocated bytes. Does not decrease after a deallocation.
    total_bytes: u64,
    // /// Number of currently allocated blocks. Decreases after a deallocation.
    // curr_blocks: usize,
    // /// Number of currently allocated bytes. Decreases after a deallocation.
    // curr_bytes: usize,
}

#[no_coverage]
fn get_allocation_stats() -> AllocationsStats {
    unsafe {
        AllocationsStats {
            total_blocks: ALLOC_STATS.total_blocks.load(Ordering::SeqCst),
            total_bytes: ALLOC_STATS.total_bytes.load(Ordering::SeqCst),
            // curr_blocks: ALLOC_STATS.curr_blocks.load(Ordering::SeqCst),
            // curr_bytes: ALLOC_STATS.curr_bytes.load(Ordering::SeqCst),
        }
    }
}
