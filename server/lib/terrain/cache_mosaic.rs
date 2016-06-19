use cgmath;
use fnv;
use lru_cache;
use std;

use common::voxel;

#[derive(Clone, PartialEq)]
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

pub type Cache<T> = lru_cache::LruCache<Key, T, std::hash::BuildHasherDefault<fnv::FnvHasher>>;

pub struct T<Material> {
  pub mosaic          : Box<voxel::mosaic::T<Material> + Send>,
  pub density         : Cache<f32>,
  pub normal          : Cache<cgmath::Vector3<f32>>,
  pub mosaic_density  : Cache<f32>,
  pub mosaic_material : Cache<Option<Material>>,
}

pub fn new<Material>(mosaic: Box<voxel::mosaic::T<Material> + Send>) -> T<Material> {
  T {
    mosaic          : mosaic,
    density         : lru_cache::LruCache::with_hash_state(1 << 10, Default::default()),
    normal          : lru_cache::LruCache::with_hash_state(1 << 10, Default::default()),
    mosaic_density  : lru_cache::LruCache::with_hash_state(1 << 10, Default::default()),
    mosaic_material : lru_cache::LruCache::with_hash_state(1 << 10, Default::default()),
  }
}

fn get_or_init<T: Clone, F>(cache: &mut Cache<T>, k: Key, f: F) -> T where
  F: FnOnce() -> T
{
  if let Some(x) = cache.get_mut(&k) {
    return x.clone()
  }

  let x = f();
  cache.insert(k, x.clone());
  x
}

impl<Material> voxel::field::T for T<Material> {
  fn density(&mut self, p: &cgmath::Point3<f32>) -> f32 {
    let mosaic = &mut self.mosaic;
    get_or_init(
      &mut self.density,
      Key(*p),
      || voxel::field::T::density(mosaic, p),
    )
  }

  fn normal(&mut self, p: &cgmath::Point3<f32>) -> cgmath::Vector3<f32> {
    let mosaic = &mut self.mosaic;
    get_or_init(
      &mut self.normal,
      Key(*p),
      || voxel::field::T::normal(mosaic, p),
    )
  }
}

impl<Material> voxel::mosaic::T<Material> for T<Material> where Material: Clone {
  fn density(&mut self, p: &cgmath::Point3<f32>) -> f32 {
    let mosaic = &mut self.mosaic;
    get_or_init(
      &mut self.mosaic_density,
      Key(*p),
      || voxel::mosaic::T::density(mosaic, p),
    )
  }

  fn material(&mut self, p: &cgmath::Point3<f32>) -> Option<Material> {
    let mosaic = &mut self.mosaic;
    get_or_init(
      &mut self.mosaic_material,
      Key(*p),
      || voxel::mosaic::T::material(mosaic, p),
    )
  }
}
