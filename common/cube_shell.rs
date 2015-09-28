//! Functions and structures for interacting with cubic shells.

use range_abs::range_abs;
use std::cmp::{min, max};
use std::iter::range_inclusive;
use block_position::BlockPosition;

#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use cgmath::{Point, Vector3};
#[cfg(test)]
use test::{Bencher, black_box};

#[inline]
// TODO: This should return an iterator.
/// Generate the set of points corresponding to the surface of a cube made of voxels.
pub fn cube_shell(center: &BlockPosition, radius: i32) -> Vec<BlockPosition> {
  let mut shell = Vec::new();

  if radius == 0 {
    shell.push(*center);
    return shell;
  }

  macro_rules! add_square(
    ($dxs: expr, $dys: expr, $dzs: expr) => (
      for dx in $dxs {
        for dy in $dys {
          for dz in $dzs {
            shell.push(BlockPosition::new(
              center.as_pnt().x + dx,
              center.as_pnt().y + dy,
              center.as_pnt().z + dz,
            ));
          }
        }
      }
    );
  );

  add_square!(
    [-radius, radius].iter().cloned(),
    range_abs(radius),
    range_abs(radius)
  );
  add_square!(
    range_abs(radius - 1),
    [-radius, radius].iter().cloned(),
    range_abs(radius)
  );
  add_square!(
    range_abs(radius - 1),
    range_abs(radius - 1),
    [-radius, radius].iter().cloned()
  );

  shell
}

// TODO: This should return an iterator.
// TODO: This should diff cube_shells instead of whole cubes.
/// Return the blocks that are present in a cube of radius `radius` centered at `from`,
/// but not in one centered at `to`.
pub fn cube_diff(
  from: &BlockPosition,
  to: &BlockPosition,
  radius: i32,
) -> Vec<BlockPosition> {
  let mut ret = Vec::new();

  macro_rules! add_square(
    ($xs: expr, $ys: expr, $zs: expr) => (
      for x in $xs {
        for y in $ys {
          for z in $zs {
            ret.push(BlockPosition::new(x, y, z));
          }
        }
      }
    );
  );

  let from = *from.as_pnt();
  let to = *to.as_pnt();

  add_square!(
    if from.x < to.x {
      range_inclusive(from.x - radius, min(from.x + radius, to.x - radius - 1))
    } else {
      range_inclusive(max(from.x - radius, to.x + radius + 1), from.x + radius)
    },
    range_inclusive(from.y - radius, from.y + radius),
    range_inclusive(from.z - radius, from.z + radius)
  );

  add_square!(
    if from.x < to.x {
      range_inclusive(to.x - radius, from.x + radius)
    } else {
      range_inclusive(from.x - radius, to.x + radius)
    },
    if from.y < to.y {
      range_inclusive(from.y - radius, min(from.y + radius, to.y - radius - 1))
    } else {
      range_inclusive(max(from.y - radius, to.y + radius + 1), from.y + radius)
    },
    range_inclusive(from.z - radius, from.z + radius)
  );

  add_square!(
    if from.x < to.x {
      range_inclusive(to.x - radius, from.x + radius)
    } else {
      range_inclusive(from.x - radius, to.x + radius)
    },
    if from.y < to.y {
      range_inclusive(to.y - radius, from.y + radius)
    } else {
      range_inclusive(from.y - radius, to.y + radius)
    },
    if from.z < to.z {
      range_inclusive(from.z - radius, min(from.z + radius, to.z - radius - 1))
    } else {
      range_inclusive(max(from.z - radius, to.z + radius + 1), from.z + radius)
    }
  );

  ret
}

#[cfg(test)]
/// the number of elements in a cube shell
pub fn cube_shell_area(radius: i32) -> u32 {
  assert!(radius >= 0);
  if radius == 0 {
    return 1;
  }

  let width = 2 * radius + 1;
  // volume of the cube
  let outer = width * width * width;
  // volume of the cube of radius r - 1.
  let inner = (width - 2) * (width - 2) * (width - 2);
  (outer - inner) as u32
}

#[test]
// Check that the surface area is correct for a few different shells.
fn test_surface_area() {
  let centers = [
    BlockPosition::new(0, 0, 0),
    BlockPosition::new(2, 1, -4),
  ];
  for center in centers.iter() {
    for radius in 1..5 {
      assert_eq!(
        cube_shell(center, radius as i32).len() as u32,
        cube_shell_area(radius)
      );
    }
  }
}

#[test]
fn test_simple_shell() {
  let center = BlockPosition::new(2, 0, -3);
  let radius = 1;

  let expected: HashSet<BlockPosition> = [
      BlockPosition::new( 0,  0,  1),
      BlockPosition::new( 0,  0, -1),
      BlockPosition::new( 0,  1,  0),
      BlockPosition::new( 0,  1,  1),
      BlockPosition::new( 0,  1, -1),
      BlockPosition::new( 0, -1,  0),
      BlockPosition::new( 0, -1,  1),
      BlockPosition::new( 0, -1, -1),
      BlockPosition::new( 1,  0,  0),
      BlockPosition::new( 1,  0,  1),
      BlockPosition::new( 1,  0, -1),
      BlockPosition::new( 1,  1,  0),
      BlockPosition::new( 1,  1,  1),
      BlockPosition::new( 1,  1, -1),
      BlockPosition::new( 1, -1,  0),
      BlockPosition::new( 1, -1,  1),
      BlockPosition::new( 1, -1, -1),
      BlockPosition::new(-1,  0,  0),
      BlockPosition::new(-1,  0,  1),
      BlockPosition::new(-1,  0, -1),
      BlockPosition::new(-1,  1,  0),
      BlockPosition::new(-1,  1,  1),
      BlockPosition::new(-1,  1, -1),
      BlockPosition::new(-1, -1,  0),
      BlockPosition::new(-1, -1,  1),
      BlockPosition::new(-1, -1, -1),
    ]
    .iter()
    .map(|p| p.clone() + center.as_pnt().to_vec())
    .collect();

  let actual = cube_shell(&center, radius);
  assert_eq!(expected, actual.into_iter().collect());
}

#[test]
fn test_shell_no_dups() {
  let center = BlockPosition::new(2, 0, -3);
  let radius = 2;
  let expected = cube_shell(&center, radius);
  let actual: HashSet<BlockPosition> = expected.iter().map(|&x| x).collect();
  assert_eq!(expected.len(), actual.len());
}

#[test]
fn test_simple_diff() {
  let from = BlockPosition::new(2, 0, -3);
  let to = from + Vector3::new(-1, 2, 0);
  let radius = 1;

  let expected: HashSet<BlockPosition> = [
      BlockPosition::new( 1,  0,  0),
      BlockPosition::new( 1,  1,  0),
      BlockPosition::new( 1, -1,  0),
      BlockPosition::new( 1,  0,  1),
      BlockPosition::new( 1,  1,  1),
      BlockPosition::new( 1, -1,  1),
      BlockPosition::new( 1,  0, -1),
      BlockPosition::new( 1,  1, -1),
      BlockPosition::new( 1, -1, -1),

      BlockPosition::new( 0, -1,  0),
      BlockPosition::new( 0,  0,  0),
      BlockPosition::new( 1, -1,  0),
      BlockPosition::new( 1,  0,  0),
      BlockPosition::new(-1, -1,  0),
      BlockPosition::new(-1,  0,  0),
      BlockPosition::new( 0, -1, -1),
      BlockPosition::new( 0,  0, -1),
      BlockPosition::new( 1, -1, -1),
      BlockPosition::new( 1,  0, -1),
      BlockPosition::new(-1, -1, -1),
      BlockPosition::new(-1,  0, -1),
      BlockPosition::new( 0, -1,  1),
      BlockPosition::new( 0,  0,  1),
      BlockPosition::new( 1, -1,  1),
      BlockPosition::new( 1,  0,  1),
      BlockPosition::new(-1, -1,  1),
      BlockPosition::new(-1,  0,  1),
    ]
    .iter()
    .map(|p| p.clone() + from.as_pnt().to_vec())
    .collect();

  let actual = cube_diff(&from, &to, radius);
  assert_eq!(expected, actual.into_iter().collect());
}

#[test]
fn test_diff_no_dups() {
  let from = BlockPosition::new(2, 0, -3);
  let to = from + Vector3::new(-1, 2, 0);
  let radius = 2;

  let expected = cube_diff(&from, &to, radius);
  let actual: HashSet<BlockPosition> = expected.iter().map(|&x| x).collect();
  assert_eq!(expected.len(), actual.len());
}

#[test]
fn test_no_diff() {
  let center = BlockPosition::new(-5, 1, -7);
  let radius = 3;
  assert!(cube_diff(&center, &center, radius).is_empty());
}

#[bench]
fn simple_shell_bench(_: &mut Bencher) {
  black_box(cube_shell(&BlockPosition::new(0, 0, 0), 400));
}

#[bench]
fn simple_diff_bench(_: &mut Bencher) {
  black_box(cube_diff(&BlockPosition::new(-1, 1, -1), &BlockPosition::new(0, 0, 0), 400));
}
