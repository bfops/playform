pub use common::voxel::T;
pub use common::voxel::storage;
pub use common::voxel::Material;
pub use voxel_data::impls::surface_vertex::T::*;
pub use voxel_data::impls::surface_vertex::{of_field, unwrap};

pub mod bounds {
  pub use common::voxel::bounds::*;

  pub mod set {
    use fnv::FnvHasher;
    use std;

    pub type T = std::collections::HashSet<super::T, std::hash::BuildHasherDefault<FnvHasher>>;

    pub fn new() -> T {
      std::collections::HashSet::with_hasher(Default::default())
    }
  }
}
