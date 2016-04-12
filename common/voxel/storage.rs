//! Datatype for storing voxel data.

use cgmath;

use ::voxel::bounds;
use ::fnv_map;

type ByPosition<T> = fnv_map::T<cgmath::Point3<i32>, T>;
// TODO: lg_size should be i32.
type ByLgSize<T> = fnv_map::T<i16, T>;

/// Type returned by `entry`.
pub type Entry<'a, V> = fnv_map::Entry<'a, cgmath::Point3<i32>, V>;

/// Voxel storage data type. This is backed by HashMaps, not an SVO, so it's a lot more compact and cache-friendly.
#[derive(Debug, Clone)]
pub struct T<V> {
  voxels: ByLgSize<ByPosition<V>>,
}

impl<V> T<V> {
  fn by_lg_size(&self, lg_size: i16) -> Option<&ByPosition<V>> {
    self.voxels.get(&lg_size)
  }

  fn by_lg_size_mut(&mut self, lg_size: i16) -> Option<&mut ByPosition<V>> {
    self.voxels.get_mut(&lg_size)
  }

  fn by_lg_size_or_insert(&mut self, lg_size: i16) -> &mut ByPosition<V> {
    self.voxels.entry(lg_size).or_insert_with(fnv_map::new)
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
    voxels: fnv_map::new(),
  }
}
