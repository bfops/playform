//! Datatype for storing voxel data.

use cgmath;
use std;

use ::voxel;

type ByPosition<T> = std::collections::HashMap<cgmath::Point3<i32>, T>;
// TODO: lg_size should be i32.
type ByLgSize<T> = std::collections::HashMap<i16, T>;

/// Type returned by `entry`.
pub type Entry<'a> = std::collections::hash_map::Entry<'a, cgmath::Point3<i32>, voxel::T>;

/// Voxel storage data type. This is backed by HashMaps, not an SVO, so it's a lot more compact and cache-friendly.
#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
pub struct T {
  voxels: ByLgSize<ByPosition<voxel::T>>,
}

impl T {
  fn by_lg_size(&self, lg_size: i16) -> Option<&ByPosition<voxel::T>> {
    self.voxels.get(&lg_size)
  }

  fn by_lg_size_or_insert(&mut self, lg_size: i16) -> &mut ByPosition<voxel::T> {
    self.voxels.entry(lg_size).or_insert_with(std::collections::HashMap::new)
  }

  /// Get an entry for in-place manipulation.
  pub fn entry<'a>(&'a mut self, voxel: &voxel::bounds::T) -> Entry<'a> {
    let p = cgmath::Point3::new(voxel.x, voxel.y, voxel.z);
    self.by_lg_size_or_insert(voxel.lg_size).entry(p)
  }

  /// Return a voxel if it's found.
  pub fn get<'a>(&'a self, voxel: &voxel::bounds::T) -> Option<&'a voxel::T> {
    let p = cgmath::Point3::new(voxel.x, voxel.y, voxel.z);
    self.by_lg_size(voxel.lg_size).and_then(|x| x.get(&p))
  }

  /// Apply a voxel brush.
  pub fn brush<Material, Mosaic, Generate, OnVoxelUpdate>(
    &mut self,
    _brush: &mut voxel::brush::T<Mosaic>,
    _generate: &mut Generate,
    _on_voxel_update: &mut OnVoxelUpdate,
  ) where
    Mosaic: voxel::mosaic::T<Material>,
    Generate: FnMut(&voxel::bounds::T) -> Option<voxel::T>,
    OnVoxelUpdate: FnMut(&voxel::T, &voxel::bounds::T),
  {
    panic!("TODO");
  }

  /// Cast a ray through the contents of this tree.
  pub fn cast_ray<'a, Act, R>(
    &'a self,
    _ray: &cgmath::Ray3<f32>,
    _act: &mut Act,
  ) -> Option<R> where
    Act: FnMut(voxel::bounds::T, &'a voxel::T) -> Option<R>,
  {
    None
  }
}

#[allow(missing_docs)]
pub fn new() -> T {
  T {
    voxels: std::collections::HashMap::new(),
  }
}
