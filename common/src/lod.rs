//! Structs for keeping track of terrain level of detail.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ops::Add;
use block_position::BlockPosition;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// A strongly-typed index into various LOD-indexed arrays.
/// 0 is the highest LOD.
pub struct LODIndex(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Level of detail a block can be loaded at.
pub enum LOD {
  /// Variable detail as an index into various LOD arrays.
  LodIndex(LODIndex),
  /// No detail: an invisible solid block that can be loaded synchronously.
  Placeholder,
}

impl PartialOrd for LOD {
  #[inline(always)]
  fn partial_cmp(&self, other: &LOD) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for LOD {
  #[inline(always)]
  fn cmp(&self, other: &LOD) -> Ordering {
    match (*self, *other) {
      (LOD::Placeholder, LOD::Placeholder) => Ordering::Equal,
      (LOD::Placeholder, LOD::LodIndex(_)) => Ordering::Less,
      (LOD::LodIndex(_), LOD::Placeholder) => Ordering::Greater,
      (LOD::LodIndex(idx1), LOD::LodIndex(idx2)) =>
        // A greater level of detail is a lower index, so invert the result of the index comparison.
        match idx1.cmp(&idx2) {
          Ordering::Less => Ordering::Greater,
          Ordering::Greater => Ordering::Less,
          ord => ord,
        }
    }
  }
}

/// Data structure to keep track of a position's owners, requested LODs, and current LOD.
pub struct LODMap {
  loaded: HashMap<BlockPosition, BlockLoadState>,
}

impl LODMap {
  #[allow(missing_docs)]
  pub fn new() -> LODMap {
    LODMap {
      loaded: HashMap::new(),
    }
  }

  /// Find out what LOD is up at a `position`.
  pub fn get<'a>(
    &'a self,
    position: &BlockPosition,
    owner: OwnerId,
  ) -> Option<(Option<LOD>, &'a Vec<(OwnerId, LOD)>)> {
    self.loaded.get(position).map(|bls| {
      let p = bls.owner_lods.iter().position(|&(o, _)| o == owner);
      let lod = p.map(|p| bls.owner_lods[p].1);
      (lod, &bls.owner_lods)
    })
  }

  // TODO: Can probably get rid of the LODChange returns; we only assert with em.

  /// Acquire/update an owner's handle in `position`.
  /// Returns (owner's previous LOD, LOD change if the location's max LOD changes).
  pub fn insert(
    &mut self,
    position: BlockPosition,
    lod: LOD,
    owner: OwnerId,
  ) -> (Option<LOD>, Option<LODChange>) {
    match self.loaded.entry(position) {
      Entry::Vacant(entry) => {
        entry.insert(BlockLoadState {
          owner_lods: vec!((owner, lod)),
          loaded_lod: lod,
        });

        (
          None,
          Some(LODChange {
            loaded: None,
            desired: Some(lod),
          }),
        )
      },
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();

        let prev_lod;
        match block_load_state.owner_lods.iter().position(|&(o, _)| o == owner) {
          None => {
            block_load_state.owner_lods.push((owner, lod));
            prev_lod = None;
          },
          Some(position) => {
            let &mut (_, ref mut cur_lod) = block_load_state.owner_lods.get_mut(position).unwrap();
            prev_lod = Some(*cur_lod);
            if lod == *cur_lod {
              return (prev_lod, None);
            }
            *cur_lod = lod;
          },
        };

        let (_, new_lod) = *block_load_state.owner_lods.iter().max_by(|&&(_, x)| x).unwrap();

        if new_lod == block_load_state.loaded_lod {
          // Already loaded at the right LOD.
          return (prev_lod, None);
        }

        let loaded_lod = Some(block_load_state.loaded_lod);
        block_load_state.loaded_lod = new_lod;

        (
          prev_lod,
          Some(LODChange {
            loaded: loaded_lod,
            desired: Some(new_lod),
          }),
        )
      },
    }
  }

  /// Release an owner's handle on `position`.
  /// Returns (owner's previous LOD, LOD change if the location's LOD changes).
  pub fn remove(
    &mut self,
    position: BlockPosition,
    owner: OwnerId,
  ) -> (Option<LOD>, Option<LODChange>) {
    match self.loaded.entry(position) {
      Entry::Vacant(_) => (None, None),
      Entry::Occupied(mut entry) => {
        let mut remove = false;
        let r = {
          let mut r = || {
            let block_load_state = entry.get_mut();

            let prev_lod;
            match block_load_state.owner_lods.iter().position(|&(o, _)| o == owner) {
              None => {
                return (None, None);
              },
              Some(position) => {
                let (_, lod) = block_load_state.owner_lods.swap_remove(position);
                prev_lod = Some(lod);
              },
            };

            let loaded_lod = block_load_state.loaded_lod;

            let new_lod;
            match block_load_state.owner_lods.iter().max_by(|&&(_, x)| x) {
              None => {
                remove = true;
                return (
                  prev_lod,
                  Some(LODChange {
                    desired: None,
                    loaded: Some(loaded_lod),
                  }),
                )
              },
              Some(&(_, lod)) => {
                new_lod = lod;
              },
            }

            if new_lod == loaded_lod {
              // Already loaded at the right LOD.
              return (prev_lod, None);
            }

            block_load_state.loaded_lod = new_lod;

            (
              prev_lod,
              Some(LODChange {
                loaded: Some(loaded_lod),
                desired: Some(new_lod),
              }),
            )
          };
          r()
        };

        if remove {
          entry.remove();
        }

        r
      },
    }
  }
}

/// A before and after LOD struct.
pub struct LODChange {
  /// The target LOD.
  pub desired: Option<LOD>,
  /// Currently-loaded LOD
  pub loaded: Option<LOD>,
}

/// These are used to identify the owners of terrain load operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct OwnerId(u32);

impl Add<u32> for OwnerId {
  type Output = OwnerId;

  fn add(self, rhs: u32) -> OwnerId {
    let OwnerId(id) = self;
    OwnerId(id + rhs)
  }
}

struct BlockLoadState {
  /// The LOD indexes requested by each owner of this block.
  // TODO: Change this back to a HashMap once initial capacity is zero for those.
  pub owner_lods: Vec<(OwnerId, LOD)>,
  pub loaded_lod: LOD,
}
