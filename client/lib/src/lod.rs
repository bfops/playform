//! Structs for keeping track of terrain level of detail.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A strongly-typed index into various LOD-indexed arrays.
/// 0 is the highest LOD.
/// Ordering is "backwards": x > y means that x is bigger (lower level of detail) than y.
pub struct T(pub u32);
