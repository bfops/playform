//! std hashset with FnvHasher parameter.

use fnv;
use std;
use std::hash::Hash;

#[allow(missing_docs)]
pub type T<V> = std::collections::HashSet<V, std::hash::BuildHasherDefault<fnv::FnvHasher>>;

#[allow(missing_docs)]
pub fn new<V: Eq + Hash>() -> T<V> {
  std::collections::HashSet::with_hasher(Default::default())
}
