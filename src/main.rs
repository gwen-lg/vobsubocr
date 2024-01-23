use anyhow::Context;
use chrono::Local;
use clap::Parser;
use log::LevelFilter;
use vobsubocr::{run, MemTracker, Opt, MEM_STATS};

// Enable only on feature
use tracking_allocator::{AllocationGroupToken, AllocationRegistry, Allocator};

use std::{alloc::System, fs::File};

#[global_allocator]
static GLOBAL: Allocator<System> = Allocator::system();

fn main() -> anyhow::Result<()> {
    let global_frame_view = puffin::GlobalFrameView::default();
    puffin::set_scopes_on(true);

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

    profiling::finish_frame!();
    write_perf_file(global_frame_view);

    res
}

fn write_perf_file(global_frame_view: puffin::GlobalFrameView) {
    let now = Local::now().format("%Y-%m-%d-%T").to_string();
    let filename = format!("perf/capture_{}.puffin", now);
    let mut file = File::create(filename).unwrap();
    let frame_view = global_frame_view.lock();
    frame_view.write(&mut file).unwrap();
}
