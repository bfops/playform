//! std hashmap with FnvHasher parameter.

use fnv;
use std;
use std::hash::Hash;

pub use std::collections::hash_map::Entry;

#[allow(missing_docs)]
pub type T<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<fnv::FnvHasher>>;

#[allow(missing_docs)]
pub fn new<K: Eq + Hash, V>() -> T<K, V> {
  std::collections::HashMap::with_hasher(Default::default())
}
