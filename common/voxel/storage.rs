//! Datatype for storing voxel data.

use cgmath;
use std;

use ::voxel::bounds;
use ::fnv_map;

#[allow(missing_docs)]
pub type ByPosition<T> = fnv_map::T<cgmath::Point3<i32>, T>;
#[allow(missing_docs)]
// TODO: lg_size should be i32.
pub type ByLgSize<T> = Vec<(i16, T)>;

/// Type returned by `entry`.
pub type Entry<'a, V> = fnv_map::Entry<'a, cgmath::Point3<i32>, V>;

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
      let mut next = (lg_size, fnv_map::new());
      for cur in &mut self.by_lg_size[posn .. len] {
        std::mem::swap(cur, &mut next);
      }
      self.by_lg_size.push(next);
    }

    &mut self.by_lg_size[posn].1
  }

  /// Get an entry for in-place manipulation.
  pub fn entry<'a>(&'a mut self, voxel: &bounds::T) -> Entry<'a, V> {
    let p = cgmath::Point3::new(voxel.x, voxel.y, voxel.z);
    self.by_lg_size_or_insert(voxel.lg_size).entry(p)
  }

  /// Return a voxel if it's found.
  pub fn get<'a>(&'a self, voxel: &bounds::T) -> Option<&'a V> {
    let p = cgmath::Point3::new(voxel.x, voxel.y, voxel.z);
    self.by_lg_size(voxel.lg_size).and_then(|x| x.get(&p))
  }

  /// Return a voxel if it's found.
  pub fn remove(&mut self, voxel: &bounds::T) -> Option<V> {
    let p = cgmath::Point3::new(voxel.x, voxel.y, voxel.z);
    self.by_lg_size_mut(voxel.lg_size).and_then(|x| x.remove(&p))
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
