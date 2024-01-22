use anyhow::Context;
use clap::Parser;
use log::LevelFilter;
use vobsubocr::{run, MemTracker, Opt, MEM_STATS};

// Enable only on feature
use tracking_allocator::{AllocationGroupToken, AllocationRegistry, Allocator};

use std::alloc::System;

#[global_allocator]
static GLOBAL: Allocator<System> = Allocator::system();

fn main() -> anyhow::Result<()> {
    let mem_tracker = MemTracker::new();
    AllocationRegistry::set_global_tracker(mem_tracker)
        .expect("no other global tracker should be set yet");

    AllocationRegistry::enable_tracking();

    simple_logger::SimpleLogger::new()
        .without_timestamps()
        .with_level(LevelFilter::Warn)
        .env()
        .init()
        .unwrap();
    let mut local_token =
        AllocationGroupToken::register().expect("failed to register allocation group");
    let local_guard = local_token.enter();

    let opt = Opt::parse();
    let res = run(&opt).with_context(|| {
        format!(
            "Could not convert '{}' to 'srt'.",
            opt.input.clone().display()
        )
    });

    drop(local_guard);
    AllocationRegistry::disable_tracking();
    MEM_STATS.print_mem_stats();
    res
}
