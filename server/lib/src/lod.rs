//! Structs for keeping track of terrain level of detail.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ops::Add;

use common::voxel;

pub use self::T::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Level of detail a block can be loaded at.
pub enum T {
  /// Variable detail as an index into various T arrays.
  Full,
  /// No detail: an invisible solid block that can be loaded synchronously.
  Placeholder,
}

impl PartialOrd for T {
  fn partial_cmp(&self, other: &T) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for T {
  fn cmp(&self, other: &T) -> Ordering {
    match (*self, *other) {
      (T::Placeholder, T::Placeholder) => Ordering::Equal,
      (T::Placeholder, T::Full) => Ordering::Less,
      (T::Full, T::Placeholder) => Ordering::Greater,
      (T::Full, T::Full) => Ordering::Equal,
    }
  }
}

/// Data structure to keep track of a position's owners, requested LODs, and current T.
pub struct Map {
  loaded: HashMap<voxel::bounds::T, BlockLoadState>,
}

impl Map {
  #[allow(missing_docs)]
  pub fn new() -> Map {
    Map {
      loaded: HashMap::new(),
    }
  }

  /// Find out what T is up at a `position`.
  pub fn get<'a>(
    &'a self,
    position: &voxel::bounds::T,
    owner: OwnerId,
  ) -> Option<(Option<T>, &'a Vec<(OwnerId, T)>)> {
    self.loaded.get(position).map(|bls| {
      let p = bls.owner_lods.iter().position(|&(o, _)| o == owner);
      let lod = p.map(|p| bls.owner_lods[p].1);
      (lod, &bls.owner_lods)
    })
  }

  // TODO: Can probably get rid of the LODChange returns; we only assert with em.

  /// Acquire/update an owner's handle in `position`.
  /// Returns (owner's previous T, T change if the location's max T changes).
  pub fn insert(
    &mut self,
    position: voxel::bounds::T,
    lod: T,
    owner: OwnerId,
  ) -> (Option<T>, Option<LODChange>) {
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
          // Already loaded at the right T.
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
  /// Returns (owner's previous T, T change if the location's T changes).
  pub fn remove(
    &mut self,
    position: voxel::bounds::T,
    owner: OwnerId,
  ) -> (Option<T>, Option<LODChange>) {
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
              // Already loaded at the right T.
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

/// A before and after T struct.
pub struct LODChange {
  /// The target T.
  pub desired: Option<T>,
  /// Currently-loaded T
  pub loaded: Option<T>,
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
  /// The T indexes requested by each owner of this block.
  // TODO: Change this back to a HashMap once initial capacity is zero for those.
  pub owner_lods: Vec<(OwnerId, T)>,
  pub loaded_lod: T,
}
