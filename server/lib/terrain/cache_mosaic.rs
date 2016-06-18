use cgmath;
use std;

use common::fnv_map;
use common::voxel;

#[derive(PartialEq)]
pub struct Key(cgmath::Point3<f32>);

impl Eq for Key {}

impl std::hash::Hash for Key {
  fn hash<H>(&self, state: &mut H) where H: std::hash::Hasher {
    unsafe {
      let as_slice: *const cgmath::Point3<u32> = std::mem::transmute(self);
      (*as_slice).hash(state)
    }
  }
}

pub type Cache<X> = fnv_map::T<Key, X>;

pub struct T<Material> {
  pub mosaic: Box<voxel::mosaic::T<Material> + Send>,
  pub cache_field_density: Cache<f32>,
  pub cache_field_normal: Cache<cgmath::Vector3<f32>>,
  pub cache_mosaic_density: Cache<f32>,
  pub cache_mosaic_material: Cache<Option<Material>>,
}

pub fn new<Material>(mosaic: Box<voxel::mosaic::T<Material> + Send>) -> T<Material> {
  T {
    mosaic: mosaic,
    cache_field_density: fnv_map::new(),
    cache_field_normal: fnv_map::new(),
    cache_mosaic_density: fnv_map::new(),
    cache_mosaic_material: fnv_map::new(),
  }
}

impl<Material> voxel::field::T for T<Material> {
  fn density(&mut self, p: &cgmath::Point3<f32>) -> f32 {
    let mosaic = &mut self.mosaic;
    *self.cache_field_density
      .entry(Key(*p))
      .or_insert_with(|| voxel::field::T::density(mosaic, p))
  }

  fn normal(&mut self, p: &cgmath::Point3<f32>) -> cgmath::Vector3<f32> {
    let mosaic = &mut self.mosaic;
    *self.cache_field_normal
      .entry(Key(*p))
      .or_insert_with(|| voxel::field::T::normal(mosaic, p))
  }
}

impl<Material> voxel::mosaic::T<Material> for T<Material> where Material: Clone {
  fn density(&mut self, p: &cgmath::Point3<f32>) -> f32 {
    let mosaic = &mut self.mosaic;
    *self.cache_mosaic_density
      .entry(Key(*p))
      .or_insert_with(|| voxel::mosaic::T::density(mosaic, p))
  }

  fn material(&mut self, p: &cgmath::Point3<f32>) -> Option<Material> {
    let mosaic = &mut self.mosaic;
    self.cache_mosaic_material
      .entry(Key(*p))
      .or_insert_with(|| voxel::mosaic::T::material(mosaic, p))
      .clone()
  }
}
