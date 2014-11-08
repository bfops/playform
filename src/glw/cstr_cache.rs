//! A cache for CStrings, since converting to one from a rust string takes an
//! allocation.
//!
//! Note that strings are never deallocated, and are per-thread.
use std::collections::hash_map::{ HashMap, Occupied, Vacant };
use std::c_str::CString;
use std::mem;

/// A set of CStrings, associated with rust strings. This makes efficiently
/// interfacing with opengl functions which take strings, easier.
#[deriving(Send, Clone)]
pub struct CStringCache {
  cache: HashMap<&'static str, CString>,
}

impl CStringCache {
  /// Creates a new, empty `StringCache`.
  pub fn new() -> CStringCache {
    CStringCache { cache: HashMap::new() }
  }

  /// Retrieves a C string for a given rust string. This will only allocate
  /// if this is the first time that particular string has been converted.
  ///
  /// Nothing is ever expired from the cache.
  pub fn convert<'a>(&'a mut self, s: &'static str) -> &'a CString {
    let ret: &'a mut CString =
      match self.cache.entry(s) {
        Vacant(entry) => entry.set(s.to_c_str()),
        Occupied(entry) => entry.into_mut(),
      };

    // TODO(cgaebel): Needed because values differ in mutability. There's
    // gotta be a safer way of expressing this.
    unsafe { mem::transmute(ret) }
  }
}
