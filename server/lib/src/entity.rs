mod types {
  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
  pub struct Terrain;

  #[allow(missing_docs)]
  #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, RustcEncodable, RustcDecodable)]
  pub struct Misc;
}

#[allow(missing_docs)]
pub mod id {
  pub use common::entity::id::*;

  #[allow(missing_docs)]
  pub type Terrain = T<super::types::Terrain>;
  #[allow(missing_docs)]
  pub type Misc = T<super::types::Misc>;
}
