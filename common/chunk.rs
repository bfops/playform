//! Chunk type

use cgmath;

use voxel;

const WIDTH: usize = 1 << 6;

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
#[allow(missing_docs)]
pub struct Position {
  coords        : cgmath::Point3<i32>,
  lg_voxel_size : i16,
}

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
#[allow(missing_docs)]
pub struct T(Vec<voxel::T>);

impl T {
  fn idx(&self, p: &cgmath::Point3<i32>) -> usize {
    (p.x as usize * WIDTH + p.y as usize) * WIDTH + p.z as usize
  }

  #[allow(missing_docs)]
  pub fn get<'a>(&'a self, p: &cgmath::Point3<i32>) -> &'a voxel::T {
    let idx = self.idx(p);
    &self.0[idx]
  }

  #[allow(missing_docs)]
  pub fn get_mut<'a>(&'a mut self, p: &cgmath::Point3<i32>) -> &'a voxel::T {
    let idx = self.idx(p);
    &mut self.0[idx]
  }
}
