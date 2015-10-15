//! This data structure emits messages to maintain its surrounding blocks in a desired
//! loaded state (e.g. to keep player surroundings loaded, or to keep unloaded blocks
//! solid near the player).

use block_position;
use block_position::BlockPosition;
use cube_shell::cube_diff;
use std::collections::VecDeque;
use stopwatch;

mod surroundings_iter {
  use cgmath::{Point3};
  use std;

  use cube_shell::cube_shell;

  struct CubeShellClosure {
    center: Point3<i32>,
  }

  impl FnOnce<(i32,)> for CubeShellClosure {
    type Output = std::vec::IntoIter<Point3<i32>>;
    extern "rust-call" fn call_once(self, radius: (i32,)) -> Self::Output {
      cube_shell(&self.center, radius.0).into_iter()
    }
  }

  impl FnMut<(i32,)> for CubeShellClosure {
    extern "rust-call" fn call_mut(&mut self, radius: (i32,)) -> Self::Output {
      cube_shell(&self.center, radius.0).into_iter()
    }
  }

  pub type T = 
    std::iter::FlatMap<
      std::ops::Range<i32>, 
      std::vec::IntoIter<Point3<i32>>,
      CubeShellClosure,
    >;

  pub fn new(center: &Point3<i32>, max_distance: i32) -> T {
    (0 .. max_distance).flat_map(CubeShellClosure {
      center: center.clone(),
    })
  }
}

#[allow(missing_docs)]
/// The type of message emitted by `SurroundingsLoader`. This stream of messages maintains
/// an owner's desired surroundings.
pub enum LoadType {
  Load,
  Unload,
  /// Load only if already loaded
  Update,
}

/// Iteratively load BlockPositions in cube-shaped layers around the some point.
/// That point can be updated with calls to `update`.
/// What "load" exactly means depends on the closures provided.
pub struct SurroundingsLoader {
  last_position: Option<BlockPosition>,

  max_load_distance: i32,
  to_load: Option<surroundings_iter::T>,

  to_recheck: VecDeque<BlockPosition>,
  // The distances to the switches between LODs.
  lod_thresholds: Vec<i32>,
}

impl SurroundingsLoader {
  #[allow(missing_docs)]
  pub fn new(
    max_load_distance: i32,
    lod_thresholds: Vec<i32>,
  ) -> SurroundingsLoader {
    assert!(max_load_distance >= 0);

    SurroundingsLoader {
      last_position: None,

      to_load: None,
      max_load_distance: max_load_distance,

      to_recheck: VecDeque::new(),
      lod_thresholds: lod_thresholds,
    }
  }

  /// Update the center point around which we load, and load some more blocks.
  pub fn updates(&mut self, position: BlockPosition) -> Updates {
    let position_changed = Some(position) != self.last_position;
    if position_changed {
      stopwatch::time("surroundings_loader.extend", || {
        self.to_load = Some(surroundings_iter::new(position.as_pnt(), self.max_load_distance));
        self.last_position.map(|last_position| {
          for &distance in &self.lod_thresholds {
            self.to_recheck.extend(
              cube_diff(&last_position.as_pnt(), &position.as_pnt(), distance).into_iter()
              .map(|p| BlockPosition::of_pnt(&p))
            );
          }
          self.to_recheck.extend(
            cube_diff(&last_position.as_pnt(), &position.as_pnt(), self.max_load_distance).into_iter()
              .map(|p| BlockPosition::of_pnt(&p))
          );
        });

        self.last_position = Some(position);
      })
    }

    Updates {
      loader: self,
      position: position,
    }
  }
}

unsafe impl Send for SurroundingsLoader {}

/// Iterator for the updates from a SurroundingsLoader.
pub struct Updates<'a> {
  loader: &'a mut SurroundingsLoader,
  position: BlockPosition,
}

impl<'a> Iterator for Updates<'a> {
  type Item = (BlockPosition, LoadType);

  fn next(&mut self) -> Option<Self::Item> {
    stopwatch::time("surroundings_loader.next", || {
      if let Some(block_position) = self.loader.to_recheck.pop_front() {
        let distance = block_position::distance(&self.position, &block_position);
        if distance > self.loader.max_load_distance {
          Some((block_position, LoadType::Unload))
        } else {
          Some((block_position, LoadType::Update))
        }
      } else {
        self.loader.to_load.as_mut().unwrap().next()
          .map(|block_position| {
            let block_position = BlockPosition::of_pnt(&block_position);
            (block_position, LoadType::Load)
          })
      }
    })
  }
}
