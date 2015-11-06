//! Voxel implementation for terrain

use isosurface_extraction;
use voxel_data;

pub use voxel_data::bounds;
pub use voxel_data::impls::surface_vertex::T::*;
pub use voxel_data::impls::surface_vertex::of_field;
pub use voxel_data::impls::surface_vertex::unwrap;

#[allow(missing_docs)]
pub type T = voxel_data::impls::surface_vertex::T<Material>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, RustcEncodable, RustcDecodable)]
#[allow(missing_docs)]
/// Terrain materials
pub enum Material {
  Empty = 0,
  Terrain = 1,
  Bark = 2,
  Leaves = 3,
  Stone = 4,
}

#[allow(missing_docs)]
pub mod tree {
  use voxel_data;

  pub use voxel_data::tree::TreeBody::*;
  pub type T = voxel_data::tree::T<super::T>;
  pub type TreeBody = voxel_data::tree::TreeBody<super::T>;
  pub type Branches = voxel_data::tree::Branches<super::T>;
}

impl isosurface_extraction::dual_contouring::material::T for Material {
  fn is_opaque(&self) -> bool {
    *self != Material::Empty
  }
}
