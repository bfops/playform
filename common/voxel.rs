//! Voxel implementation for terrain

use isosurface_extraction;
use voxel_data;

pub use voxel_data::bounds;
pub use voxel_data::impls::surface_vertex::T::*;
pub use voxel_data::impls::surface_vertex::of_field;
pub use voxel_data::impls::surface_vertex::unwrap;

#[allow(missing_docs)]
pub type T = voxel_data::impls::surface_vertex::T<Material>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
/// Terrain materials
pub enum Material {
  Empty = 0,
  Terrain = 1,
  Bark = 2,
  Leaves = 3,
  Stone = 4,
  Marble = 5,
}

#[allow(missing_docs)]
pub mod tree {
  use voxel_data;

  pub type T = voxel_data::tree::T<super::T>;
  pub type Inner = voxel_data::tree::Inner<super::T>;
  pub type Branches = voxel_data::tree::Branches<super::T>;

  pub fn new() -> T {
    voxel_data::tree::new()
  }
}

#[allow(missing_docs)]
pub mod brush {
  pub use voxel_data::brush::*;
}

#[allow(missing_docs)]
pub mod field {
  pub use voxel_data::field::*;
}

#[allow(missing_docs)]
pub mod mosaic {
  pub use voxel_data::mosaic::*;
}

impl isosurface_extraction::dual_contouring::material::T for Material {
  fn is_opaque(&self) -> bool {
    *self != Material::Empty
  }
}
