pub use common::voxel::T;
pub use common::voxel::storage;
pub use common::voxel::Material;
pub use voxel_data::impls::surface_vertex::T::*;
pub use voxel_data::impls::surface_vertex::{of_field, unwrap};

pub mod bounds {
  pub use common::voxel::bounds::*;

  pub mod set {
    use common::fnv_set;

    pub type T = fnv_set::T<super::T>;

    pub fn new() -> T {
      fnv_set::new()
    }
  }
}
