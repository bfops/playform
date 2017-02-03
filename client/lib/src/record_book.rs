//! Record stats about the program execution

/// end-to-end timing of loading a single chunk
#[derive(Debug, Clone, Copy)]
pub struct ChunkLoad {
  /// when the initial request happened
  pub time_requested_ns : u64,
  /// when the server responded
  pub response_time_ns  : u64,
  /// when the voxels were loaded into the client terrain cache
  pub stored_time_ns    : u64,
  /// when the voxels were turned into mesh and enqueued into vram queues
  pub loaded_time_ns    : u64,
}

#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct T {
  pub chunk_loads: Vec<ChunkLoad>,
}

#[allow(missing_docs)]
pub fn new() -> T {
  T {
    chunk_loads: Vec::new(),
  }
}

/// to avoid locking, keep separate record books per thread
pub mod thread_local {
  use super::*;

  use std;

  thread_local!(static THREAD_LOCAL: std::cell::RefCell<T> = std::cell::RefCell::new(new()));

  #[allow(missing_docs)]
  pub fn push_chunk_load(x: ChunkLoad) {
    THREAD_LOCAL.with(|t| t.borrow_mut().chunk_loads.push(x));
  }

  #[allow(missing_docs)]
  pub fn clone() -> T {
    THREAD_LOCAL.with(|t| t.borrow().clone())
  }
}
