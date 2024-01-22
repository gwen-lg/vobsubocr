use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{
        atomic::{AtomicU32, AtomicUsize, Ordering},
        Mutex,
    },
};

use tracking_allocator::{AllocationGroupId, AllocationTracker};

pub struct MemStats {
    pub total_alloc_count: AtomicU32,
    pub current_alloc_count: AtomicU32,
    pub max_alloc_count: AtomicU32,
    pub total_alloc_size: AtomicUsize,
    pub current_alloc_size: AtomicUsize,
    pub max_alloc_size: AtomicUsize,
}

pub static MEM_STATS: MemStats = MemStats {
    total_alloc_count: AtomicU32::new(0),
    current_alloc_count: AtomicU32::new(0),
    max_alloc_count: AtomicU32::new(0),
    current_alloc_size: AtomicUsize::new(0),
    max_alloc_size: AtomicUsize::new(0),
    total_alloc_size: AtomicUsize::new(0),
};

impl MemStats {
    pub fn print_mem_stats(&self) {
        let total_alloc_count = MEM_STATS.total_alloc_count.load(Ordering::Relaxed);
        let max_alloc_count = MEM_STATS.max_alloc_count.load(Ordering::Relaxed);
        let total_alloc_size = MEM_STATS.total_alloc_size.load(Ordering::Relaxed);
        let max_alloc_size = MEM_STATS.max_alloc_size.load(Ordering::Relaxed);
        if total_alloc_count > 0 {
            //HACK
            println!(
                //*writer,
                "Stats:\n\
                \ttotal alloc count : {total_alloc_count}\n\
                \tmax alloc : {max_alloc_count}\n\
                \ttotal alloc size: {total_alloc_size}\n\
                \tmax alloc size : {max_alloc_size}",
            );
        }
    }
}

pub struct MemTracker {
    writer: Mutex<BufWriter<File>>,
}

impl MemTracker {
    pub fn new() -> Self {
        let file = File::create("mem_out.txt").unwrap();
        let writer = BufWriter::new(file);
        let writer = Mutex::new(writer);
        Self { writer }
    }
}

impl Drop for MemTracker {
    fn drop(&mut self) {
        let mut writer = self.writer.lock().unwrap();
        writer.flush().unwrap();
    }
}

// This is our tracker implementation.  You will always need to create an implementation of `AllocationTracker` in order
// to actually handle allocation events.  The interface is straightforward: you're notified when an allocation occurs,
// and when a deallocation occurs.
impl AllocationTracker for MemTracker {
    fn allocated(
        &self,
        addr: usize,
        object_size: usize,
        wrapped_size: usize,
        group_id: AllocationGroupId,
    ) {
        // Stats
        MEM_STATS.total_alloc_count.fetch_add(1, Ordering::Relaxed);
        MEM_STATS
            .current_alloc_count
            .fetch_add(1, Ordering::Relaxed);
        MEM_STATS
            .total_alloc_size
            .fetch_add(object_size, Ordering::Relaxed);
        MEM_STATS
            .current_alloc_size
            .fetch_add(object_size, Ordering::Relaxed);

        // Allocations have all the pertinent information upfront, which you may or may not want to store for further
        // analysis. Notably, deallocations also know how large they are, and what group ID they came from, so you
        // typically don't have to store much data for correlating deallocations with their original allocation.
        let mut writer = self.writer.lock().unwrap();
        writeln!(
            *writer,
            "allocation -> addr=0x{:0x} object_size={} wrapped_size={} group_id={:?}",
            addr, object_size, wrapped_size, group_id
        )
        .unwrap();
    }

    fn deallocated(
        &self,
        addr: usize,
        object_size: usize,
        wrapped_size: usize,
        source_group_id: AllocationGroupId,
        current_group_id: AllocationGroupId,
    ) {
        // Stats :
        let previous_alloc_count = MEM_STATS
            .current_alloc_count
            .fetch_sub(1, Ordering::Relaxed);
        MEM_STATS
            .max_alloc_count
            .fetch_max(previous_alloc_count, Ordering::Relaxed);

        let previous_alloc_size = MEM_STATS
            .current_alloc_size
            .fetch_sub(object_size, Ordering::Relaxed);
        MEM_STATS
            .max_alloc_size
            .fetch_max(previous_alloc_size, Ordering::Relaxed);

        // When a deallocation occurs, as mentioned above, you have full access to the address, size of the allocation,
        // as well as the group ID the allocation was made under _and_ the active allocation group ID.
        //
        // This can be useful beyond just the obvious "track how many current bytes are allocated by the group", instead
        // going further to see the chain of where allocations end up, and so on.
        let mut writer = self.writer.lock().unwrap();
        writeln!(
            *writer,
            "deallocation -> addr=0x{:0x} object_size={} wrapped_size={} source_group_id={:?} current_group_id={:?}",
            addr, object_size, wrapped_size, source_group_id, current_group_id
        ).unwrap();
    }
}
