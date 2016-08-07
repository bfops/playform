//! Chunk type

use cgmath;
use std;

use voxel;

/// Width of a chunk, in voxels
pub const WIDTH: u32 = 1 << LG_WIDTH;
/// Base-2 log of the chunk width
pub const LG_WIDTH: u16 = 5;

/// A chunk position in "chunk coordinates".
pub mod position {
  use cgmath;

  #[derive(Debug, Clone, RustcEncodable, RustcDecodable, PartialEq, Eq, Hash)]
  #[allow(missing_docs)]
  /// Positions are implicitly multiples of the chunk size, which is
  /// `WIDTH` times the voxel size.
  pub struct T {
    pub as_point : cgmath::Point3<i32>,
  }
}

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
#[allow(missing_docs)]
pub struct T {
  pub voxels : Vec<voxel::T>,
}

impl T {
  fn idx(&self, p: &cgmath::Point3<i32>) -> usize {
    (p.x as usize * WIDTH as usize + p.y as usize) * WIDTH as usize + p.z as usize
  }

  /// Get a reference to the voxel at a point specified relative to the lowest
  /// corner of the chunk.
  pub fn get<'a>(&'a self, p: &cgmath::Point3<i32>) -> &'a voxel::T {
    let idx = self.idx(p);
    &self.voxels[idx]
  }

  /// Get a mutable reference to the voxel at a point specified relative to the
  /// lowest corner of the chunk.
  pub fn get_mut<'a>(&'a mut self, p: &cgmath::Point3<i32>) -> &'a mut voxel::T {
    let idx = self.idx(p);
    &mut self.voxels[idx]
  }
}

/// Iterate through the voxels in this chunk.
pub fn voxels<'a>(chunk: &'a T, position: &'a position::T, lg_voxel_size: i16) -> Voxels<'a> {
  Voxels::new(chunk, position, lg_voxel_size)
}

/// Return an iterator for the bounds of the voxels in a chunk.
pub fn voxel_bounds(p: &position::T, lg_voxel_size: i16) -> VoxelBounds {
  VoxelBounds::new(p, lg_voxel_size)
}

/// Construct a chunk from a position and an initialization callback.
pub fn of_callback<F>(position: &position::T, lg_voxel_size: i16, mut f: F) -> T
  where F: FnMut(voxel::bounds::T) -> voxel::T
{
  assert!(lg_voxel_size <= 0 || lg_voxel_size as u16 <= LG_WIDTH);

  let mut voxels = Vec::new();

  let samples = 1 << (LG_WIDTH as i16 - lg_voxel_size);
  for x in 0 .. samples {
  for y in 0 .. samples {
  for z in 0 .. samples {
    let bounds =
      voxel::bounds::T {
        x: position.as_point.x + x,
        y: position.as_point.y + y,
        z: position.as_point.z + z,
        lg_size: lg_voxel_size,
      };
    voxels.push(f(bounds));
  }}}

  T {
    voxels: voxels,
  }
}

/// An iterator for the bounds of the voxels inside a chunk.
pub struct VoxelBounds<'a> {
  position      : &'a position::T,
  lg_voxel_size : i16,
  current       : cgmath::Point3<u8>,
  done          : bool,
}

impl<'a> VoxelBounds <'a> {
  #[allow(missing_docs)]
  pub fn new<'b:'a>(position: &'b position::T, lg_voxel_size: i16) -> Self {
    VoxelBounds {
      position      : position,
      lg_voxel_size : lg_voxel_size,
      current       : cgmath::Point3::new(0, 0, 0),
      done          : false,
    }
  }
}

impl<'a> std::iter::Iterator for VoxelBounds<'a> {
  type Item = voxel::bounds::T;
  fn next(&mut self) -> Option<Self::Item> {
    if self.done {
      return None
    }

    let r =
      Some(
        voxel::bounds::T {
          x       : WIDTH as i32 * self.position.as_point.x + self.current.x as i32,
          y       : WIDTH as i32 * self.position.as_point.y + self.current.y as i32,
          z       : WIDTH as i32 * self.position.as_point.z + self.current.z as i32,
          lg_size : self.lg_voxel_size,
        },
      );

    self.current.x += 1;
    if (self.current.x as u32) < WIDTH { return r }
    self.current.x = 0;

    self.current.y += 1;
    if (self.current.y as u32) < WIDTH { return r }
    self.current.y = 0;

    self.current.z += 1;
    if (self.current.z as u32) < WIDTH { return r }
    self.done = true;

    r
  }
}

/// An iterator for the voxels inside a chunk.
pub struct Voxels<'a> {
  chunk         : &'a T,
  position      : &'a position::T,
  lg_voxel_size : i16,
  current       : cgmath::Point3<u8>,
  done          : bool,
}

impl<'a> Voxels <'a> {
  #[allow(missing_docs)]
  pub fn new<'b: 'a>(chunk: &'b T, position: &'b position::T, lg_voxel_size: i16) -> Self {
    Voxels {
      chunk         : chunk,
      position      : position,
      lg_voxel_size : lg_voxel_size,
      current       : cgmath::Point3::new(0, 0, 0),
      done          : false,
    }
  }
}

impl<'a> std::iter::Iterator for Voxels<'a> {
  type Item = (voxel::bounds::T, voxel::T);
  fn next(&mut self) -> Option<Self::Item> {
    if self.done {
      return None
    }

    let x = self.position.as_point.x + self.current.x as i32;
    let y = self.position.as_point.y + self.current.y as i32;
    let z = self.position.as_point.z + self.current.z as i32;
    let r =
      Some((
        voxel::bounds::T { x: x, y: y, z: z, lg_size: self.lg_voxel_size },
        *self.chunk.get(&cgmath::Point3::new(x, y, z)),
      ));

    self.current.z += 1;
    if (self.current.z as u32) < WIDTH { return r }
    self.current.z = 0;

    self.current.y += 1;
    if (self.current.y as u32) < WIDTH { return r }
    self.current.y = 0;

    self.current.x += 1;
    if (self.current.x as u32) < WIDTH { return r }
    self.done = true;

    r
  }
}
