use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
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

  /// Returns: (new LOD, previous LOD)
  pub fn increase_lod(
    &mut self,
    position: BlockPosition,
    new_lod: LOD,
    owner: OwnerId,
  ) -> Option<LODChange> {
    match self.loaded.entry(position) {
      Entry::Vacant(entry) => {
        let mut owner_lods = HashMap::new();
        owner_lods.insert(owner, new_lod);
        entry.insert(BlockLoadState {
          owner_lods: owner_lods,
          loaded_lod: new_lod,
        });

        Some(LODChange {
          loaded: None,
          desired: Some(new_lod),
        })
      },
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();

        match block_load_state.owner_lods.entry(owner) {
          Entry::Occupied(mut entry) => {
            if new_lod <= *entry.get() {
              return None;
            }
            entry.insert(new_lod);
          },
          Entry::Vacant(entry) => {
            entry.insert(new_lod);
          }
        }

        let new_lod = *block_load_state.owner_lods.values().max_by(|x| *x).unwrap();

        if new_lod == block_load_state.loaded_lod {
          // Already loaded at the right LOD.
          return None;
        }

        let loaded_lod = Some(block_load_state.loaded_lod);
        block_load_state.loaded_lod = new_lod;

        Some(LODChange {
          loaded: loaded_lod,
          desired: Some(new_lod),
        })
      },
    }
  }

  pub fn decrease_lod(
    &mut self,
    position: BlockPosition,
    new_lod: Option<LOD>,
    owner: OwnerId,
  ) -> Option<LODChange> {
    match self.loaded.entry(position) {
      Entry::Vacant(_) => None,
      Entry::Occupied(mut entry) => {
        let block_load_state = entry.get_mut();

        match block_load_state.owner_lods.entry(owner) {
          Entry::Vacant(_) => return None,
          Entry::Occupied(mut entry) => {
            match new_lod {
              None => {
                entry.remove();
              },
              Some(new_lod) => {
                if new_lod >= *entry.get() {
                  return None;
                }
                entry.insert(new_lod);
              }
            }
          },
        }

        let loaded_lod = block_load_state.loaded_lod;

        let new_lod;
        match block_load_state.owner_lods.values().max_by(|x| *x) {
          None => {
            return Some(LODChange {
              desired: None,
              loaded: Some(loaded_lod),
            })
          },
          Some(&lod) => {
            new_lod = lod;
          },
        }

        if new_lod == loaded_lod {
          // Already loaded at the right LOD.
          return None;
        }

        block_load_state.loaded_lod = new_lod;

        Some(LODChange {
          loaded: Some(loaded_lod),
          desired: Some(new_lod),
        })
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
  pub owner_lods: HashMap<OwnerId, LOD>,
  pub loaded_lod: LOD,
}
