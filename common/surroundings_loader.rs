//! This data structure emits messages to maintain its surrounding blocks in a desired
//! loaded state (e.g. to keep player surroundings loaded, or to keep unloaded blocks
//! solid near the player).

use cgmath::Point3;
use std::cmp::max;
use std::collections::VecDeque;
use stopwatch;

use cube_shell::cube_diff;

mod surroundings_iter {
  use cgmath::{Point3};
  use std;

  use cube_shell::cube_shell;

  pub struct CubeShellClosure {
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
      center: *center,
    })
  }
}

#[allow(missing_docs)]
/// The type of message emitted by `T`. This stream of messages maintains
/// an owner's desired surroundings.
pub enum LoadType {
  Load,
  Unload,
  /// Load only if already loaded
  Downgrade,
}

/// Iteratively load points cube-shaped layers around the some point.
/// That point can be updated with calls to `update`.
/// What "load" exactly means depends on the closures provided.
pub struct T {
  last_position: Option<Point3<i32>>,

  max_load_distance: i32,
  to_load: Option<surroundings_iter::T>,

  to_recheck: VecDeque<Point3<i32>>,
  // The distances to the switches between LODs.
  lod_thresholds: Vec<i32>,
}

#[allow(missing_docs)]
pub fn new(
  max_load_distance: i32,
  lod_thresholds: Vec<i32>,
) -> T {
  assert!(max_load_distance >= 0);

  T {
    last_position: None,

    to_load: None,
    max_load_distance: max_load_distance,

    to_recheck: VecDeque::new(),
    lod_thresholds: lod_thresholds,
  }
}

impl T {
  /// Update the center point around which we load, and load some more blocks.
  pub fn updates(&mut self, position: &Point3<i32>) -> Updates {
    let position_changed = self.last_position != Some(*position);
    if position_changed {
      stopwatch::time("surroundings_loader.extend", || {
        self.to_load = Some(surroundings_iter::new(&position, self.max_load_distance));
        self.last_position.map(|last_position| {
          for &distance in &self.lod_thresholds {
            self.to_recheck.extend(
              cube_diff(&last_position, &position, distance).into_iter()
            );
          }
          self.to_recheck.extend(
            cube_diff(&last_position, &position, self.max_load_distance).into_iter()
          );
        });

        self.last_position = Some(*position);
      })
    }

    Updates {
      loader: self,
      position: *position,
    }
  }
}

unsafe impl Send for T {}

/// Iterator for the updates from a T.
pub struct Updates<'a> {
  loader: &'a mut T,
  position: Point3<i32>,
}

/// Find the minimum cube shell radius it would take from one point to intersect the other.
pub fn distance_between(p1: &Point3<i32>, p2: &Point3<i32>) -> i32 {
  let dx = (p1.x - p2.x).abs();
  let dy = (p1.y - p2.y).abs();
  let dz = (p1.z - p2.z).abs();
  max(max(dx, dy), dz)
}

impl<'a> Iterator for Updates<'a> {
  type Item = (Point3<i32>, LoadType);

  fn next(&mut self) -> Option<Self::Item> {
    stopwatch::time("surroundings_loader.next", || {
      if let Some(position) = self.loader.to_recheck.pop_front() {
        let distance = distance_between(&self.position, &position);
        if distance > self.loader.max_load_distance {
          Some((position, LoadType::Unload))
        } else {
          Some((position, LoadType::Downgrade))
        }
      } else {
        self.loader.to_load.as_mut().unwrap().next()
          .map(|position| (position, LoadType::Load))
      }
    })
  }
}
