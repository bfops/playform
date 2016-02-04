#[derive(Debug, Clone, Copy)]
pub struct BlockLoad
{
  pub requested_at: u64,
  pub responded_at: u64,
  pub processed_at: u64,
  pub loaded_at: u64,
}

#[derive(Debug, Clone)]
pub struct T
{
  pub block_loads: Vec<BlockLoad>,
}

pub fn new() -> T
{
  T {
    block_loads: Vec::new(),
  }
}

pub mod thread_local {
  use super::*;

  use std;

  thread_local!(static THREAD_LOCAL: std::cell::RefCell<T> = std::cell::RefCell::new(new()));

  pub fn push_block_load(x: BlockLoad)
  {
    THREAD_LOCAL.with(|t| t.borrow_mut().block_loads.push(x));
  }

  pub fn clone() -> T
  {
    THREAD_LOCAL.with(|t| t.borrow().clone())
  }
}
