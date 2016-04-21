//! Datatype for storing voxel data.

use cgmath;
use std;

use ::voxel::bounds;

/// Key for hashing voxel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key(pub cgmath::Point3<i32>);

#[allow(derive_hash_xor_eq)]
impl std::hash::Hash for Key {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    let x: &[u8] =
      unsafe {
        std::slice::from_raw_parts(
          std::mem::transmute(self),
          std::mem::size_of::<Key>(),
        )
      };
    state.write(x);
  }
}

/// A special Hasher for 3D points that interleaves the bottom 21 bits of each coordinate.
pub struct Hasher {
  x: i32,
  y: i32,
  z: i32,
}

impl std::default::Default for Hasher {
  fn default() -> Hasher {
    Hasher { x: 0, y: 0, z: 0 }
  }
}

impl std::hash::Hasher for Hasher {
  fn write(&mut self, bytes: &[u8]) {
    debug_assert!(bytes.len() == 12);
    unsafe {
      let bytes: *const i32 = bytes.as_ptr() as *const i32;
      self.x = *bytes;
      self.y = *bytes.offset(1);
      self.z = *bytes.offset(2);
    }
  }

  #[allow(identity_op)]
  fn finish(&self) -> u64 {
    let mut r: u64 = 0;
    for i in 0..21 { r = r | (((self.x as u64 & (1 << i)) << (i << 1)) << 0); }
    for i in 0..21 { r = r | (((self.y as u64 & (1 << i)) << (i << 1)) << 1); }
    for i in 0..21 { r = r | (((self.z as u64 & (1 << i)) << (i << 1)) << 2); }
    r
  }
}

#[allow(missing_docs)]
pub type ByPosition<T> = std::collections::HashMap<Key, T, std::hash::BuildHasherDefault<Hasher>>;
#[allow(missing_docs)]
// TODO: lg_size should be i32.
pub type ByLgSize<T> = Vec<(i16, T)>;

/// Type returned by `entry`.
pub type Entry<'a, V> = std::collections::hash_map::Entry<'a, Key, V>;

/// Voxel storage data type. This is backed by HashMaps, not an SVO, so it's a lot more compact and cache-friendly.
#[derive(Debug, Clone)]
pub struct T<V> {
  #[allow(missing_docs)]
  pub by_lg_size: ByLgSize<ByPosition<V>>,
}

impl<V> T<V> {
  fn by_lg_size(&self, lg_size: i16) -> Option<&ByPosition<V>> {
    self.by_lg_size.iter()
      .filter(|&&(l, _)| l == lg_size)
      .map(|&(_, ref v)| v)
      .next()
  }

  fn by_lg_size_mut(&mut self, lg_size: i16) -> Option<&mut ByPosition<V>> {
    self.by_lg_size.iter_mut()
      .filter(|&&mut (l, _)| l == lg_size)
      .map(|&mut (_, ref mut v)| v)
      .next()
  }

  fn by_lg_size_or_insert(&mut self, lg_size: i16) -> &mut ByPosition<V> {
    let posn =
      self.by_lg_size.iter()
        .take_while(|&&(l, _)| l < lg_size)
        .count();

    let len = self.by_lg_size.len();
    if posn == len || self.by_lg_size[posn].0 != lg_size {
      let mut next = (lg_size, std::collections::HashMap::with_hasher(Default::default()));
      for cur in &mut self.by_lg_size[posn .. len] {
        std::mem::swap(cur, &mut next);
      }
      self.by_lg_size.push(next);
    }

    &mut self.by_lg_size[posn].1
  }

  /// Get an entry for in-place manipulation.
  pub fn entry<'a>(&'a mut self, voxel: &bounds::T) -> Entry<'a, V> {
    let k = Key(cgmath::Point3::new(voxel.x, voxel.y, voxel.z));
    self.by_lg_size_or_insert(voxel.lg_size).entry(k)
  }

  /// Return a voxel if it's found.
  pub fn get<'a>(&'a self, voxel: &bounds::T) -> Option<&'a V> {
    let k = Key(cgmath::Point3::new(voxel.x, voxel.y, voxel.z));
    self.by_lg_size(voxel.lg_size).and_then(|x| x.get(&k))
  }

  /// Return a voxel if it's found.
  pub fn remove(&mut self, voxel: &bounds::T) -> Option<V> {
    let k = Key(cgmath::Point3::new(voxel.x, voxel.y, voxel.z));
    self.by_lg_size_mut(voxel.lg_size).and_then(|x| x.remove(&k))
  }

  /// Apply a voxel brush.
  pub fn brush<Material, Mosaic, Generate, OnVoxelUpdate>(
    &mut self,
    _brush: &mut ::voxel::brush::T<Mosaic>,
    _generate: &mut Generate,
    _on_voxel_update: &mut OnVoxelUpdate,
  ) where
    Mosaic: ::voxel::mosaic::T<Material>,
    Generate: FnMut(&bounds::T) -> Option<V>,
    OnVoxelUpdate: FnMut(&V, &bounds::T),
  {
    panic!("TODO");
  }

  /// Cast a ray through the contents of this tree.
  pub fn cast_ray<'a, Act, R>(
    &'a self,
    _ray: &cgmath::Ray3<f32>,
    _act: &mut Act,
  ) -> Option<R> where
    Act: FnMut(bounds::T, &'a V) -> Option<R>,
  {
    None
  }
}

#[allow(missing_docs)]
pub fn new<V>() -> T<V> {
  T {
    by_lg_size: Vec::new(),
  }
}
