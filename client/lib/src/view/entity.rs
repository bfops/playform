//! Entity types from common, extended with client-specific entity types.

mod types {
  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
  pub struct Terrain;

  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
  pub struct Grass;
}

#[allow(missing_docs)]
pub mod id {
  pub use common::entity::id::*;

  #[allow(missing_docs)]
  pub type Terrain = T<super::types::Terrain>;
  #[allow(missing_docs)]
  pub type Grass = T<super::types::Grass>;
}
