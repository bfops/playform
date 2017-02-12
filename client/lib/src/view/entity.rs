use common;

mod types {
  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
  pub struct Terrain;

  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
  pub struct Mob;

  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
  pub struct Grass;
}

#[allow(missing_docs)]
pub mod id {
  pub use common::entity::id::*;

  #[allow(missing_docs)]
  pub type Terrain = T<super::types::Terrain>;
  #[allow(missing_docs)]
  pub type Mob = T<super::types::Mob>;
  #[allow(missing_docs)]
  pub type Grass = T<super::types::Grass>;
}
