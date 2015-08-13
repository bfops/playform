use cgmath::{Aabb, Point, Vector, Vector3, Ray3};
use std::mem;
use std::ops::{Deref, DerefMut};

use voxel;

mod raycast;

#[derive(Debug)]
pub struct T<Voxel> {
  /// The tree extends 2^lg_size in each direction.
  /// i.e. the total width is 2^(lg_size + 1).
  lg_size: u8,
  /// Force the top level to always be branches;
  /// it saves a branch in the grow logic.
  contents: Branches<Voxel>,
}

#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Branches<Voxel> {
  // xyz ordering
  // This isn't an array because we can't move out of an array.

  lll: TreeBody<Voxel>,
  llh: TreeBody<Voxel>,
  lhl: TreeBody<Voxel>,
  lhh: TreeBody<Voxel>,
  hll: TreeBody<Voxel>,
  hlh: TreeBody<Voxel>,
  hhl: TreeBody<Voxel>,
  hhh: TreeBody<Voxel>,
}

/// The main, recursive, tree-y part of the voxel tree.
#[derive(Debug, PartialEq, Eq)]
pub enum TreeBody<Voxel> {
  Empty,
  Leaf(Voxel),
  Branch(Box<Branches<Voxel>>),
}

impl<Voxel> Branches<Voxel> {
  pub fn empty() -> Branches<Voxel> {
    Branches {
      lll: TreeBody::Empty,
      llh: TreeBody::Empty,
      lhl: TreeBody::Empty,
      lhh: TreeBody::Empty,
      hll: TreeBody::Empty,
      hlh: TreeBody::Empty,
      hhl: TreeBody::Empty,
      hhh: TreeBody::Empty,
    }
  }

  pub fn as_array(&self) -> &[[[TreeBody<Voxel>; 2]; 2]; 2] {
    unsafe {
      mem::transmute(self)
    }
  }

  pub fn as_array_mut(&mut self) -> &mut [[[TreeBody<Voxel>; 2]; 2]; 2] {
    unsafe {
      mem::transmute(self)
    }
  }
}

fn brush_overlaps(voxel: &voxel::Bounds, brush: &::voxel::brush::Bounds) -> bool {
  if voxel.lg_size >= 0 {
    let min =
      Vector3::new(
        voxel.x << voxel.lg_size,
        voxel.y << voxel.lg_size,
        voxel.z << voxel.lg_size,
      );
    min.x < brush.max().x &&
    min.y < brush.max().y &&
    min.z < brush.max().z &&
    {
      let max = min.add_s(1 << voxel.lg_size);
      brush.min().x < max.x &&
      brush.min().y < max.y &&
      brush.min().z < max.z &&
      true
    }
  } else {
    let lg_size = -voxel.lg_size;
    let max =
      Vector3::new(
        brush.max().x << lg_size,
        brush.max().y << lg_size,
        brush.max().z << lg_size,
      );
    voxel.x < max.x &&
    voxel.y < max.y &&
    voxel.z < max.z &&
    {
      let min =
        Vector3::new(
          brush.min().x << lg_size,
          brush.min().y << lg_size,
          brush.min().z << lg_size,
        );
      min.x <= voxel.x &&
      min.y <= voxel.y &&
      min.z <= voxel.z &&
      true
    }
  }
}

impl<Voxel> TreeBody<Voxel> {
  pub fn remove<Brush>(
    &mut self,
    bounds: &voxel::Bounds,
    brush: &Brush,
    brush_bounds: &::voxel::brush::Bounds,
  ) where
    Brush: voxel::brush::T<Voxel=Voxel>,
  {
    if !brush_overlaps(bounds, brush_bounds) {
      return
    }

    match self {
      &mut TreeBody::Branch(ref mut branches) => {
        // Bounds of the lowest branch
        let bounds = voxel::Bounds::new(bounds.x << 1, bounds.y << 1, bounds.z << 1, bounds.lg_size - 1);

        macro_rules! recurse(($branch: ident, $update_bounds: expr) => {{
          let mut bounds = bounds;
          $update_bounds(&mut bounds);
          branches.$branch.remove(&bounds, brush, brush_bounds);
        }});
        recurse!(lll, |_|                     {                            });
        recurse!(llh, |b: &mut voxel::Bounds| {                    b.z += 1});
        recurse!(lhl, |b: &mut voxel::Bounds| {          b.y += 1          });
        recurse!(lhh, |b: &mut voxel::Bounds| {          b.y += 1; b.z += 1});
        recurse!(hll, |b: &mut voxel::Bounds| {b.x += 1                    });
        recurse!(hlh, |b: &mut voxel::Bounds| {b.x += 1;           b.z += 1});
        recurse!(hhl, |b: &mut voxel::Bounds| {b.x += 1; b.y += 1});
        recurse!(hhh, |b: &mut voxel::Bounds| {b.x += 1; b.y += 1; b.z += 1});
      },
      &mut TreeBody::Empty => {},
      &mut TreeBody::Leaf(ref mut voxel) => {
        voxel::brush::T::remove(voxel, bounds, brush);
      }
    }
  }
}

impl<Voxel> T<Voxel> {
  pub fn new() -> T<Voxel> {
    T {
      lg_size: 0,
      contents: Branches::<Voxel>::empty(),
    }
  }

  /// Is this voxel (non-strictly) within an origin-centered voxel with
  /// width `2^(lg_size + 1)`?
  pub fn contains_bounds(&self, voxel: &voxel::Bounds) -> bool {
    let high;
    if voxel.lg_size >= 0 {
      high = (1 << self.lg_size) >> voxel.lg_size;
    } else {
      high = (1 << self.lg_size) << (-voxel.lg_size);
    }

    voxel.x < high &&
    voxel.y < high &&
    voxel.z < high &&
    {
      let low = -high;
      voxel.x >= low &&
      voxel.y >= low &&
      voxel.z >= low &&
      true
    }
  }

  /// Ensure that this tree can hold the provided voxel.
  pub fn grow_to_hold(&mut self, voxel: &voxel::Bounds) {
    while !self.contains_bounds(voxel) {
      // Double the bounds in every direction.
      self.lg_size += 1;

      // Pull out `self.contents` so we can move out of it.
      let contents = mem::replace(&mut self.contents, Branches::<Voxel>::empty());

      // We re-construct the tree with bounds twice the size (but still centered
      // around the origin) by deconstructing the top level of branches,
      // creating a new doubly-sized top level, and moving the old branches back
      // in as the new top level's children. e.g. in 2D:
      //
      //                      ---------------------------
      //                      |     |     |0|     |     |
      //                      |     |     |0|     |     |
      // ---------------      ------------|0|------------
      // |  1  |0|  2  |      |     |  1  |0|  2  |     |
      // |     |0|     |      |     |     |0|     |     |
      // |------0------|      |------------0------------|
      // 000000000000000  ==> |0000000000000000000000000|
      // |------0------|      |------------0------------|
      // |     |0|     |      |     |     |0|     |     |
      // |  3  |0|  4  |      |     |  3  |0|  4  |     |
      // ---------------      |------------0------------|
      //                      |     |     |0|     |     |
      //                      |     |     |0|     |     |
      //                      ---------------------------

      macro_rules! at(
        ($c_idx:ident, $b_idx:ident) => {{
          let mut branches = Branches::<Voxel>::empty();
          branches.$b_idx = contents.$c_idx;
          TreeBody::Branch(Box::new(branches))
        }}
      );

      self.contents =
        Branches {
          lll: at!(lll, hhh),
          llh: at!(llh, hhl),
          lhl: at!(lhl, hlh),
          lhh: at!(lhh, hll),
          hll: at!(hll, lhh),
          hlh: at!(hlh, lhl),
          hhl: at!(hhl, llh),
          hhh: at!(hhh, lll),
        };
    }
  }

  fn find_mask(&self, voxel: &voxel::Bounds) -> i32 {
    // When we compare the voxel position to octree bounds to choose subtrees
    // for insertion, we'll be comparing voxel position to values of 2^n and
    // -2^n, so we can just use the position bits to branch directly.
    // This actually works for negative values too, without much wrestling:
    // we need to branch on the sign bit up front, but after that, two's
    // complement magic means the branching on bits works regardless of sign.

    let mut mask = (1 << self.lg_size) >> 1;

    // Shift everything by the voxel's lg_size, so we can compare the mask to 0
    // to know whether we're done.
    if voxel.lg_size >= 0 {
      mask = mask >> voxel.lg_size;
    } else {
      // TODO: Check for overflow.
      mask = mask << -voxel.lg_size;
    }

    mask
  }

  fn find_mut<'a, Step, E>(
    &'a mut self,
    voxel: &voxel::Bounds,
    mut step: Step,
  ) -> Result<&'a mut TreeBody<Voxel>, E> where
    Step: FnMut(&'a mut TreeBody<Voxel>) -> Result<&'a mut Branches<Voxel>, E>,
  {
    let mut mask = self.find_mask(voxel);
    let mut branches = &mut self.contents;

    macro_rules! iter(
      ($select:expr, $step:block) => {{
        let branches_temp = branches;
        let branch =
          &mut branches_temp.as_array_mut()
            [$select(voxel.x)]
            [$select(voxel.y)]
            [$select(voxel.z)]
          ;

        $step;
        // We've reached the voxel.
        if mask == 0 {
          return Ok(branch)
        }

        branches = try!(step(branch));
      }}
    );

    iter!(|x| (x >= 0) as usize, {});

    loop {
      iter!(
        |x| ((x & mask) != 0) as usize,
        // Branch through half this size next time.
        { mask = mask >> 1; }
      );
    }
  }

  #[allow(dead_code)]
  fn find<'a, Step, E>(
    &'a self,
    voxel: &voxel::Bounds,
    mut step: Step,
  ) -> Result<&'a TreeBody<Voxel>, E> where
    Step: FnMut(&'a TreeBody<Voxel>) -> Result<&'a Branches<Voxel>, E>,
  {
    let mut mask = self.find_mask(voxel);
    let mut branches = &self.contents;

    macro_rules! iter(
      ($select:expr, $step:block) => {{
        let branches_temp = branches;
        let branch =
          &branches_temp.as_array()
            [$select(voxel.x)]
            [$select(voxel.y)]
            [$select(voxel.z)]
          ;

        $step;
        // We've reached the voxel.
        if mask == 0 {
          return Ok(branch)
        }

        branches = try!(step(branch));
      }}
    );

    iter!(|x| (x >= 0) as usize, {});

    loop {
      iter!(
        |x| { ((x & mask) != 0) as usize },
        // Branch through half this size next time.
        { mask = mask >> 1; }
      );
    }
  }

  #[allow(dead_code)]
  /// Find a voxel inside this tree.
  /// If it doesn't exist, it will be created as empty.
  pub fn get_mut_or_create<'a>(&'a mut self, voxel: &voxel::Bounds) -> &'a mut TreeBody<Voxel> {
    self.grow_to_hold(voxel);
    let branch: Result<_, ()> =
      self.find_mut(voxel, |branch| { Ok(T::<Voxel>::get_mut_or_create_step(branch)) });
    branch.unwrap()
  }

  fn get_mut_or_create_step<'a>(
    branch: &'a mut TreeBody<Voxel>,
  ) -> &'a mut Branches<Voxel> {
    // "Step down" the tree.
    match *branch {
      // Branches<Voxel>; we can go straight to the branching logic.
      TreeBody::Branch(ref mut b) => b,

      // Otherwise, keep going, but we need to insert a voxel inside the
      // space occupied by the current branch.

      TreeBody::Empty => {
        // Replace this branch with 8 empty sub-branches - who's gonna notice?
        *branch = TreeBody::Branch(Box::new(Branches::<Voxel>::empty()));

        match *branch {
          TreeBody::Branch(ref mut b) => b,
          _ => unreachable!(),
        }
      },
      TreeBody::Leaf(_) => {
        // Erase this leaf and replace it with 8 empty sub-branches.
        // This behavior is pretty debatable, but we need to do something,
        // and it's easier to debug accidentally replacing a big chunk
        // with a smaller one than to debug a nop.
        *branch = TreeBody::Branch(Box::new(Branches::<Voxel>::empty()));

        match *branch {
          TreeBody::Branch(ref mut b) => b,
          _ => unreachable!(),
        }
      },
    }
  }

  #[allow(dead_code)]
  /// Find a voxel inside this tree.
  pub fn get<'a>(&'a self, voxel: &voxel::Bounds) -> Option<&'a Voxel> {
    if !self.contains_bounds(voxel) {
      return None
    }

    let get_step = |branch| {
      match branch {
        &TreeBody::Branch(ref branches) => Ok(branches.deref()),
        _ => Err(()),
      }
    };

    match self.find(voxel, get_step) {
      Ok(&TreeBody::Leaf(ref t)) => Some(t),
      _ => None,
    }
  }

  /// Find a voxel inside this tree.
  #[allow(dead_code)]
  pub fn get_mut<'a>(&'a mut self, voxel: &voxel::Bounds) -> Option<&'a mut Voxel> {
    if !self.contains_bounds(voxel) {
      return None
    }

    let get_step = |branch| {
      match branch {
        &mut TreeBody::Branch(ref mut branches) => Ok(branches.deref_mut()),
        _ => Err(()),
      }
    };

    match self.find_mut(voxel, get_step) {
      Ok(&mut TreeBody::Leaf(ref mut t)) => Some(t),
      _ => None,
    }
  }

  pub fn cast_ray<'a, Act, R>(
    &'a self,
    ray: &Ray3<f32>,
    act: &mut Act,
  ) -> Option<R>
    where
      // TODO: Does this *have* to be callback-based?
      Act: FnMut(voxel::Bounds, &'a Voxel) -> Option<R>
  {
    let coords = [
      if ray.origin.x >= 0.0 {1} else {0},
      if ray.origin.y >= 0.0 {1} else {0},
      if ray.origin.z >= 0.0 {1} else {0},
    ];
    // NB: The children are half the size of the tree itself,
    // but tree.lg_size=0 means it extends tree.lg_size=0 in *each direction*,
    // so the "actual" size of the tree as a voxel would be tree.lg_size+1.
    let child_lg_size = self.lg_size as i16;
    let mut make_bounds = |coords: [usize; 3]| {
      voxel::Bounds {
        x: coords[0] as i32 - 1,
        y: coords[1] as i32 - 1,
        z: coords[2] as i32 - 1,
        lg_size: child_lg_size,
      }
    };
    match raycast::cast_ray_branches(
      &self.contents,
      ray,
      None,
      coords,
      &mut make_bounds,
      act,
    ) {
      Ok(r) => Some(r),
      Err(_) => None,
    }
  }

  pub fn remove<Brush>(
    &mut self,
    brush: &Brush,
    brush_bounds: &::voxel::brush::Bounds,
  ) where
    Brush: voxel::brush::T<Voxel=Voxel>,
  {
    macro_rules! recurse(($branch: ident, $x: expr, $y: expr, $z: expr) => {{
      self.contents.$branch.remove(
        &voxel::Bounds::new($x, $y, $z, self.lg_size as i16),
        brush,
        brush_bounds,
      );
    }});
    recurse!(lll, -1, -1, -1);
    recurse!(llh, -1, -1,  0);
    recurse!(lhl, -1,  0, -1);
    recurse!(lhh, -1,  0,  0);
    recurse!(hll,  0, -1, -1);
    recurse!(hlh,  0, -1,  0);
    recurse!(hhl,  0,  0, -1);
    recurse!(hhh,  0,  0,  0);
  }
}

#[cfg(test)]
mod tests {
  extern crate test;

  use voxel;
  use super::{T, TreeBody};

  #[test]
  fn insert_and_lookup() {
    let mut tree: T<i32> = T::<i32>::new();
    *tree.get_mut_or_create(voxel::Bounds::new(1, 1, 1, 0)) = TreeBody::<i32>::Leaf(1);
    *tree.get_mut_or_create(voxel::Bounds::new(8, -8, 4, 0)) = TreeBody::<i32>::Leaf(2);
    *tree.get_mut_or_create(voxel::Bounds::new(2, 0, 4, 4)) = TreeBody::<i32>::Leaf(3);
    *tree.get_mut_or_create(voxel::Bounds::new(9, 0, 16, 2)) = TreeBody::<i32>::Leaf(4);
    *tree.get_mut_or_create(voxel::Bounds::new(9, 0, 16, 2)) = TreeBody::<i32>::Leaf(5);

    assert_eq!(tree.get(voxel::Bounds::new(1, 1, 1, 0)), Some(&1));
    assert_eq!(tree.get(voxel::Bounds::new(8, -8, 4, 0)), Some(&2));
    assert_eq!(tree.get(voxel::Bounds::new(9, 0, 16, 2)), Some(&5));

    assert_eq!(tree.get(voxel::Bounds::new(2, 0, 4, 4)), None);
  }

  #[test]
  fn wrong_voxel_size_is_not_found() {
    let mut tree: T<i32> = T::<i32>::new();
    *tree.get_mut_or_create(voxel::Bounds::new(4, 4, -4, 1)) = TreeBody::Leaf(1);
    assert_eq!(tree.get(voxel::Bounds::new(4, 4, -4, 0)), None);
    assert_eq!(tree.get(voxel::Bounds::new(4, 4, -4, 2)), None);
  }

  #[test]
  fn grow_is_transparent() {
    let mut tree: T<i32> = T::<i32>::new();
    *tree.get_mut_or_create(voxel::Bounds::new(1, 1, 1, 0)) = TreeBody::Leaf(1);
    tree.grow_to_hold(voxel::Bounds::new(0, 0, 0, 1));
    tree.grow_to_hold(voxel::Bounds::new(0, 0, 0, 2));
    tree.grow_to_hold(voxel::Bounds::new(-32, 32, -128, 3));

    assert_eq!(tree.get(voxel::Bounds::new(1, 1, 1, 0)), Some(&1));
  }

  #[test]
  fn simple_cast_ray() {
    let mut tree: T<i32> = T::<i32>::new();
    *tree.get_mut_or_create(voxel::Bounds::new(1, 1, 1, 0)) = TreeBody::Leaf(1);
    *tree.get_mut_or_create(voxel::Bounds::new(4, 4, 4, 0)) = TreeBody::Leaf(2);

    let actual = tree.cast_ray(
      [4.5, 3.0, 4.5],
      [0.1, 0.8, 0.1],
      // Return the first voxel we hit.
      &mut |bounds, v| Some((bounds, v)),
    );

    assert_eq!(actual, Some((voxel::Bounds::new(4, 4, 4, 0), &2)));
  }

  #[bench]
  fn simple_inserts(bencher: &mut test::Bencher) {
    let mut tree: T<i32> = T::<i32>::new();
    tree.grow_to_hold(voxel::Bounds::new(0, 0, 0, 30));
    bencher.iter(|| {
      *tree.get_mut_or_create(voxel::Bounds::new(0, 0, 0, 0)) = TreeBody::Leaf(0);
    });
    test::black_box(tree);
  }

  #[bench]
  fn bench_cast_ray(bencher: &mut test::Bencher) {
    let mut tree: T<i32> = T::<i32>::new();
    tree.grow_to_hold(voxel::Bounds::new(0, 0, 0, 30));
    *tree.get_mut_or_create(voxel::Bounds::new(1, 1, 1, 0)) = TreeBody::Leaf(1);
    *tree.get_mut_or_create(voxel::Bounds::new(4, 4, 4, 0)) = TreeBody::Leaf(2);

    bencher.iter(|| {
      let r = tree.cast_ray(
        [4.5, 3.0, 4.5],
        [0.1, 0.8, 0.1],
        // Return the first voxel we hit.
        &mut |bounds, v| Some((bounds, v)),
      );
      test::black_box(r);
    });
  }
}
