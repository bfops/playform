#[derive(Debug, Clone, Copy)]
pub struct ChunkLoad {
  pub time_requested_ns  : u64,
  pub response_time_ns : u64,
  pub stored_time_ns   : u64,
  pub loaded_time_ns   : u64,
}

#[derive(Debug, Clone)]
pub struct T {
  pub chunk_loads: Vec<ChunkLoad>,
}

pub fn new() -> T {
  T {
    chunk_loads: Vec::new(),
  }
}

pub mod thread_local {
  use super::*;

  use std;

  thread_local!(static THREAD_LOCAL: std::cell::RefCell<T> = std::cell::RefCell::new(new()));

  pub fn push_chunk_load(x: ChunkLoad) {
    THREAD_LOCAL.with(|t| t.borrow_mut().chunk_loads.push(x));
  }

  pub fn clone() -> T {
    THREAD_LOCAL.with(|t| t.borrow().clone())
  }
}
