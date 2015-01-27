use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::iter::IteratorExt;
use std::ops::Add;
use terrain_block::BlockPosition;

#[derive(Show, Clone, Copy, PartialEq, Eq)]
pub enum LOD {
  LodIndex(u32),
  // An invisible solid block
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

pub struct LODMap {
  pub loaded: HashMap<BlockPosition, BlockLoadState>,
}

impl LODMap {
  pub fn new() -> LODMap {
    LODMap {
      loaded: HashMap::new(),
    }
  }

  // Returns (previous `owner` LOD, LOD change)
  pub fn increase_lod(
    &mut self,
    position: BlockPosition,
    new_lod: LOD,
    owner: OwnerId,
  ) -> (Option<LOD>, Option<LODChange>) {
    match self.loaded.entry(position) {
      Entry::Vacant(entry) => {
        entry.insert(BlockLoadState {
          owner_lods: vec!((owner, new_lod)),
          loaded_lod: new_lod,
        });

        (
          None,
          Some(LODChange {
            loaded: None,
            desired: Some(new_lod),
          }),
        )
      },
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();

        let prev_lod;
        match block_load_state.owner_lods.iter().position(|&(o, _)| o == owner) {
          None => {
            block_load_state.owner_lods.push((owner, new_lod));
            prev_lod = None;
          },
          Some(position) => {
            let &mut (_, ref mut lod) = block_load_state.owner_lods.get_mut(position).unwrap();
            if new_lod <= *lod {
              return (None, None);
            }
            prev_lod = Some(*lod);
            *lod = new_lod;
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

  // Returns (previous `owner` LOD, LOD change)
  pub fn decrease_lod(
    &mut self,
    position: BlockPosition,
    new_lod: Option<LOD>,
    owner: OwnerId,
  ) -> (Option<LOD>, Option<LODChange>) {
    match self.loaded.entry(position) {
      Entry::Vacant(_) => (None, None),
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();

        let prev_lod;
        match block_load_state.owner_lods.iter().position(|&(o, _)| o == owner) {
          None => {
            return (None, None);
          },
          Some(position) => {
            match new_lod {
              None => {
                block_load_state.owner_lods.swap_remove(position);
                prev_lod = None;
              },
              Some(new_lod) => {
                let &mut (_, ref mut lod) =
                  block_load_state.owner_lods.get_mut(position).unwrap();
                if new_lod >= *lod {
                  return (None, None);
                }
                prev_lod = Some(*lod);
                *lod = new_lod;
              }
            }
          },
        };

        let loaded_lod = block_load_state.loaded_lod;

        let new_lod;
        match block_load_state.owner_lods.iter().max_by(|&&(_, x)| x) {
          None => {
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
      },
    }
  }
}

pub struct LODChange {
  pub desired: Option<LOD>,
  pub loaded: Option<LOD>,
}

/// These are used to identify the owners of terrain load operations.
#[derive(Copy, Clone, Show, PartialEq, Eq, Hash, Default)]
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
