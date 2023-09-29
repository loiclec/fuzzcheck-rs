use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::{SaveToStatsFolder, Sensor};

/// A [`Sensor`](crate::Sensor) that records the allocations made by the test
/// function.
///
/// It can only be used when the `#[global_allocator]` is a value of type
/// [`CountingAllocator<A>`](self::CountingAllocator).
///
/// Its [observations](crate::Sensor::Observations) are a tuple `(u64, u64)`
/// where the first element is the number of allocations performed and the
/// second element is the amount of bytes that were allocated.
///
/// # Example
///
/// ```rust
/// use std::alloc::System;
/// use fuzzcheck::{Arguments, ReasonForStopping, PoolExt};
/// use fuzzcheck::sensors_and_pools::{CountingAllocator, AllocationSensor, MaximiseObservationPool, DifferentObservations};
///
/// // set the global allocator to CountingAllocator so that the allocation sensor
/// // can retrieve allocation data
/// #[global_allocator]
/// static alloc: CountingAllocator<System> = CountingAllocator(System);
///
/// // This function fails on the very specific input `[98, 18, 9, 203, 45, 165]`.
/// // For every matching element, an integer is allocated on the heap and pushed to a vector.
/// // By trying to maximise the number of allocations, the fuzzer can incrementally find the failing input.
/// fn test_function(xs: &[u8]) -> bool {
///     if xs.len() == 6 {
///         let mut v = vec![];
///
///         if xs[0] == 98  { v.push(Box::new(0)) }
///         if xs[1] == 18  { v.push(Box::new(1)) }
///         if xs[2] == 9   { v.push(Box::new(2)) }
///         if xs[3] == 203 { v.push(Box::new(3)) }
///         if xs[4] == 45  { v.push(Box::new(4)) }
///         if xs[5] == 165 { v.push(Box::new(5)) }
///
///         v.len() != 6
///     } else {
///         true
///     }
/// }
/// let sensor = AllocationSensor::default();
///
/// // The sensor can be paired with any pool which is compatible with
/// // observations of type `(u64, u64)`. For example, we can use the following
/// // pool to maximise both elements of the tuple.
/// let pool =
///     MaximiseObservationPool::<u64>::new("alloc_blocks")
///     .and(
///         MaximiseObservationPool::<u64>::new("alloc_bytes"),
///         None,
///         DifferentObservations
///     );
///
/// // then launch fuzzcheck with this sensor and pool
/// let result = fuzzcheck::fuzz_test(test_function)
///     .default_mutator()
///     .serde_serializer()
///     .sensor_and_pool(sensor, pool)
///     .arguments(Arguments::for_internal_documentation_test())
///     .stop_after_first_test_failure(true)
///     .launch();
///
/// assert!(matches!(
///     result.reason_for_stopping,
///     ReasonForStopping::TestFailure(x)
///         if matches!(
///             x.as_slice(),
///             [98, 18, 9, 203, 45, 165]
///         )
/// ));
/// ```
#[derive(Default)]
pub struct AllocationSensor {
    start_allocs: AllocationsStats,
    end_allocs: AllocationsStats,
}

impl SaveToStatsFolder for AllocationSensor {
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        vec![]
    }
}
impl Sensor for AllocationSensor {
    type Observations = (u64, u64);

    #[coverage(off)]
    fn start_recording(&mut self) {
        self.start_allocs = get_allocation_stats();
    }

    #[coverage(off)]
    fn stop_recording(&mut self) {
        self.end_allocs = get_allocation_stats();
    }

    #[coverage(off)]
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
    #[coverage(off)]
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
    #[coverage(off)]
    fn realloc(&mut self, size: usize, shrink: bool, delta: usize) {
        self.total_blocks.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(size as u64, Ordering::Relaxed);
        if shrink {
            self.curr_bytes.fetch_sub(delta, Ordering::Relaxed);
        } else {
            self.curr_bytes.fetch_add(delta, Ordering::Relaxed);
        }
    }
    #[coverage(off)]
    fn alloc(&mut self, size: usize) {
        self.total_blocks.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(size as u64, Ordering::Relaxed);

        self.curr_blocks.fetch_add(1, Ordering::Relaxed);
        self.curr_bytes.fetch_add(size, Ordering::Relaxed);
    }

    #[coverage(off)]
    fn dealloc(&mut self, size: usize) {
        self.curr_blocks.fetch_sub(1, Ordering::Relaxed);
        self.curr_bytes.fetch_sub(size, Ordering::Relaxed);
    }
}

/// A global allocator that counts the total number of allocations as well as
/// the total number of allocated bytes.
///
/// Its only purpose is to be used with an [`AllocationSensor`].
///
/// Its argument is the underlying global allocator. For example, to use
/// with the system allocator:
/// ```
/// use std::alloc::System;
/// use fuzzcheck::sensors_and_pools::{CountingAllocator};
///
/// #[global_allocator]
/// static alloc: CountingAllocator<System> = CountingAllocator(System);
/// ```
#[derive(Debug)]
pub struct CountingAllocator<A>(pub A)
where
    A: GlobalAlloc;

unsafe impl<A> GlobalAlloc for CountingAllocator<A>
where
    A: GlobalAlloc,
{
    #[coverage(off)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.0.alloc(layout);
        if ptr.is_null() {
            return ptr;
        }
        let size = layout.size();
        ALLOC_STATS.alloc(size);
        ptr
    }

    #[coverage(off)]
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
    #[coverage(off)]
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

#[coverage(off)]
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
