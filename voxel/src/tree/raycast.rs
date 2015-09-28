use cgmath::{Point, Vector, Ray3};
use std::cmp::Ordering;

use bounds;
use tree::{TreeBody, Branches};

// Time-of-intersection. Implements `Ord` for sanity reasons;
// let's hope the floating-points are all valid.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
struct TOI(pub f32);

impl Eq for TOI {}

impl Ord for TOI {
  fn cmp(&self, other: &TOI) -> Ordering {
    self.partial_cmp(other).unwrap()
  }
}

#[derive(Debug, Copy, Clone)]
/// Information about a ray entering a voxel.
pub struct Entry {
  /// Index of a side of a rectangular-prismic voxel.
  side: usize,
  // (Roughly) when the side was intersected.
  toi: TOI,
}

impl Entry {
  pub fn from_exit(exit: Exit) -> Entry {
    Entry {
      side:
        if exit.side < 3 {
          exit.side + 3
        } else {
          exit.side - 3
        },
      toi: exit.toi,
    }
  }
}

#[derive(Debug, Copy, Clone)]
/// Information about a ray exit a voxel.
pub struct Exit {
  /// Index of a side of a rectangular-prismic voxel.
  side: usize,
  // (Roughly) when the side was intersected.
  toi: TOI,
}

// TODO: Audit all the divisions for divide-by-zeros.

#[inline]
pub fn cast_ray_branches<'a, Voxel, MakeBounds, Act, R>(
  this: &'a Branches<Voxel>,
  ray: &Ray3<f32>,
  mut entry: Option<Entry>,
  mut coords: [usize; 3],
  make_bounds: &mut MakeBounds,
  act: &mut Act,
) -> Result<R, Exit>
  where
    MakeBounds: FnMut([usize; 3]) -> bounds::T,
    Act: FnMut(bounds::T, &'a Voxel) -> Option<R>,
{
  loop {
    let child = &this.as_array()[coords[0]][coords[1]][coords[2]];
    let bounds = make_bounds(coords);

    match cast_ray(child, ray, bounds, entry, act) {
      Ok(r) => return Ok(r),
      Err(exit) => {
        let dim = exit.side % 3;
        if ray.direction[dim] < 0.0 {
          if coords[dim] == 0 {
            return Err(exit)
          }
          coords[dim] = 0;
        } else {
          if coords[dim] == 1 {
            return Err(exit)
          }
          coords[dim] = 1;
        }
        entry = Some(Entry::from_exit(exit));
      },
    }
  }
}

/// Precondition: the ray passes through `this`.
pub fn cast_ray<'a, Voxel, Act, R>(
  this: &'a TreeBody<Voxel>,
  ray: &Ray3<f32>,
  bounds: bounds::T,
  entry: Option<Entry>,
  act: &mut Act,
) -> Result<R, Exit>
  where
    Act: FnMut(bounds::T, &'a Voxel) -> Option<R>
{
  match this {
    &TreeBody::Empty => {
      // We pass through empty voxels; fall through.
    },
    &TreeBody::Branch { ref data, ref branches } => {
      match data {
        &Some(ref voxel) => {
          if let Some(r) = act(bounds, voxel) {
            return Ok(r)
          }
        },
        &None => {
          let mid = bounds.center();

          let mut make_bounds = |coords: [usize; 3]| {
            let mut bounds = bounds;
            bounds.lg_size -= 1;
            bounds.x <<= 1;
            bounds.y <<= 1;
            bounds.z <<= 1;
            bounds.x += coords[0] as i32;
            bounds.y += coords[1] as i32;
            bounds.z += coords[2] as i32;
            bounds
          };

          let entry_toi = entry.map(|entry| entry.toi.0).unwrap_or(0.0);
          let intersect = ray.origin.add_v(&ray.direction.mul_s(entry_toi));
          let coords = [
            if intersect.x >= mid.x {1} else {0},
            if intersect.y >= mid.y {1} else {0},
            if intersect.z >= mid.z {1} else {0},
          ];

          return cast_ray_branches(
            branches,
            ray,
            entry,
            coords,
            &mut make_bounds,
            act,
          )
        },
      }
    }
  };

  // We pass through this voxel; calculate the exit.

  let sides = [
    (ray.origin.x, ray.direction.x, bounds.x),
    (ray.origin.y, ray.direction.y, bounds.y),
    (ray.origin.z, ray.direction.z, bounds.z),
    (ray.origin.x, ray.direction.x, bounds.x + 1),
    (ray.origin.y, ray.direction.y, bounds.y + 1),
    (ray.origin.z, ray.direction.z, bounds.z + 1),
  ];

  let voxel_size = bounds.size();
  let next_toi = |(side, &(origin, direction, bound))| {
    let bound = bound as f32 * voxel_size;
    if direction == 0.0 {
      None
    } else {
      let toi = (bound - origin) / direction;
      if entry.map(|entry| entry.toi.0 <= toi).unwrap_or(toi >= 0.0) {
        Some(Exit {
          side: side,
          toi: TOI(toi),
        })
      } else {
        None
      }
    }
  };

  let exit =
    match entry {
      None =>
        sides.iter()
        .enumerate()
        .filter_map(next_toi)
        .min_by(|&exit| exit.toi).unwrap(),
      Some(entry) =>
        sides.iter()
        .enumerate()
        .filter(|&(i, _)| i != entry.side)
        .filter_map(next_toi)
        .min_by(|&exit| exit.toi).unwrap(),
    };
  Err(exit)
}
