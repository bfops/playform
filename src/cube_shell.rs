use range_abs::range_abs;
use terrain_block::BlockPosition;

#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use test::Bencher;

#[inline]
/// Generate the set of points corresponding to the surface of a cube made of voxels.
pub fn cube_shell(center: &BlockPosition, radius: i32) -> Vec<BlockPosition> {
  let mut shell = Vec::new();
 
  macro_rules! add_square(
    ($dxs: expr, $dys: expr, $dzs: expr) => (
      for dx in $dxs {
        for dy in $dys {
          for dz in $dzs {
            shell.push(BlockPosition::new(
              center.as_pnt3().x + dx,
              center.as_pnt3().y + dy,
              center.as_pnt3().z + dz,
            ));
          }
        }
      }
    );
  );
 
  add_square!(
    [-radius, radius].iter().map(|&dx| dx),
    range_abs(radius),
    range_abs(radius)
  );
  add_square!(
    range_abs(radius - 1),
    [-radius, radius].iter().map(|&dy| dy),
    range_abs(radius)
  );
  add_square!(
    range_abs(radius - 1),
    range_abs(radius - 1),
    [-radius, radius].iter().map(|&dz| dz)
  );
 
  shell
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
  (outer - inner) as uint
}

#[test]
// Check that the surface area is correct for a few different shells.
fn test_surface_area() {
  let centers = [
    Pnt3::new(0, 0, 0),
    Pnt3::new(2, 1, -4),
  ];
  for center in centers.iter() {
    for radius in range(1, 5) {
      assert_eq!(
        cube_shell(center, radius as int).len(),
        cube_shell_area(radius)
      );
    }
  }
}

#[test]
fn test_simple_shell() {
  let center = Pnt3::new(2, 0, -3);
  let radius = 1;

  let expected: HashSet<Pnt3<int>> = [
      Pnt3::new( 0,  0,  1),
      Pnt3::new( 0,  0, -1),
      Pnt3::new( 0,  1,  0),
      Pnt3::new( 0,  1,  1),
      Pnt3::new( 0,  1, -1),
      Pnt3::new( 0, -1,  0),
      Pnt3::new( 0, -1,  1),
      Pnt3::new( 0, -1, -1),
      Pnt3::new( 1,  0,  0),
      Pnt3::new( 1,  0,  1),
      Pnt3::new( 1,  0, -1),
      Pnt3::new( 1,  1,  0),
      Pnt3::new( 1,  1,  1),
      Pnt3::new( 1,  1, -1),
      Pnt3::new( 1, -1,  0),
      Pnt3::new( 1, -1,  1),
      Pnt3::new( 1, -1, -1),
      Pnt3::new(-1,  0,  0),
      Pnt3::new(-1,  0,  1),
      Pnt3::new(-1,  0, -1),
      Pnt3::new(-1,  1,  0),
      Pnt3::new(-1,  1,  1),
      Pnt3::new(-1,  1, -1),
      Pnt3::new(-1, -1,  0),
      Pnt3::new(-1, -1,  1),
      Pnt3::new(-1, -1, -1),
    ]
    .iter()
    .map(|p| p.clone() + center.to_vec())
    .collect();

  assert_eq!(expected, cube_shell(&center, radius).into_iter().collect());
}

#[bench]
fn simple_bench(_: &mut Bencher) {
  let _ = cube_shell(&Pnt3::new(0, 0, 0), 100);
}
