#![cfg_attr(test, feature(test))]

use cgmath::Ray3;
use std::mem;
use std::ops::{Deref, DerefMut};

use raycast;
use voxel;
use voxel::Voxel;

#[derive(Debug)]
pub struct VoxelTree {
  /// The log_2 of the tree's size.
  lg_size: u8,
  /// Force the top level to always be branches;
  /// it saves a branch in the grow logic.
  contents: Branches,
}

#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Branches {
  // xyz ordering
  // This isn't an array because we can't move out of an array.

  lll: TreeBody,
  llh: TreeBody,
  lhl: TreeBody,
  lhh: TreeBody,
  hll: TreeBody,
  hlh: TreeBody,
  hhl: TreeBody,
  hhh: TreeBody,
}

/// The main, recursive, tree-y part of the `VoxelTree`.
#[derive(Debug, PartialEq, Eq)]
pub enum TreeBody {
  Empty,
  Leaf(Voxel),
  Branch(Box<Branches>),
}

impl Branches {
  pub fn empty() -> Branches {
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

  pub fn get<'a>(&'a self, x: usize, y: usize, z: usize) -> &'a TreeBody {
    let this: &'a [[[TreeBody; 2]; 2]; 2] = unsafe {
      mem::transmute(self)
    };
    &this[x][y][z]
  }

  pub fn get_mut<'a>(&'a mut self, x: usize, y: usize, z: usize) -> &'a mut TreeBody {
    let this: &'a mut [[[TreeBody; 2]; 2]; 2] = unsafe {
      mem::transmute(self)
    };
    &mut this[x][y][z]
  }
}

impl VoxelTree {
  pub fn new() -> VoxelTree {
    VoxelTree {
      lg_size: 0,
      contents: Branches::empty(),
    }
  }

  /// Is this voxel (non-strictly) within an origin-centered voxel with
  /// size `2^lg_size`?
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
      let contents = mem::replace(&mut self.contents, Branches::empty());

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
          let mut branches = Branches::empty();
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
  ) -> Result<&'a mut TreeBody, E> where
    Step: FnMut(&'a mut TreeBody) -> Result<&'a mut Branches, E>,
  {
    let mut mask = self.find_mask(voxel);
    let mut branches = &mut self.contents;

    macro_rules! iter(
      ($select:expr, $step:block) => {{
        let branches_temp = branches;
        let x = $select(voxel.x);
        let y = $select(voxel.y);
        let z = $select(voxel.z);
        let branch = branches_temp.get_mut(x, y, z);

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

  fn find<'a, Step, E>(
    &'a self,
    voxel: &voxel::Bounds,
    mut step: Step,
  ) -> Result<&'a TreeBody, E> where
    Step: FnMut(&'a TreeBody) -> Result<&'a Branches, E>,
  {
    let mut mask = self.find_mask(voxel);
    let mut branches = &self.contents;

    macro_rules! iter(
      ($select:expr, $step:block) => {{
        let branches_temp = branches;
        let x = $select(voxel.x);
        let y = $select(voxel.y);
        let z = $select(voxel.z);
        let branch = branches_temp.get(x, y, z);

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

  /// Find a voxel inside this tree.
  /// If it doesn't exist, it will be created as empty.
  pub fn get_mut_or_create<'a>(&'a mut self, voxel: &voxel::Bounds) -> &'a mut TreeBody {
    self.grow_to_hold(voxel);
    let branch: Result<_, ()> =
      self.find_mut(voxel, |branch| { Ok(VoxelTree::get_mut_or_create_step(branch)) });
    branch.unwrap()
  }

  fn get_mut_or_create_step<'a>(
    branch: &'a mut TreeBody,
  ) -> &'a mut Branches {
    // "Step down" the tree.
    match *branch {
      // Branches; we can go straight to the branching logic.
      TreeBody::Branch(ref mut b) => b,

      // Otherwise, keep going, but we need to insert a voxel inside the
      // space occupied by the current branch.

      TreeBody::Empty => {
        // Replace this branch with 8 empty sub-branches - who's gonna notice?
        *branch = TreeBody::Branch(Box::new(Branches::empty()));

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
        *branch = TreeBody::Branch(Box::new(Branches::empty()));

        match *branch {
          TreeBody::Branch(ref mut b) => b,
          _ => unreachable!(),
        }
      },
    }
  }

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
}

#[cfg(test)]
mod tests {
  extern crate test;

  use voxel;
  use super::{VoxelTree, TreeBody};

  #[test]
  fn insert_and_lookup() {
    let mut tree: VoxelTree<i32> = VoxelTree::new();
    *tree.get_mut_or_create(voxel::Bounds::new(1, 1, 1, 0)) = TreeBody::Leaf(1);
    *tree.get_mut_or_create(voxel::Bounds::new(8, -8, 4, 0)) = TreeBody::Leaf(2);
    *tree.get_mut_or_create(voxel::Bounds::new(2, 0, 4, 4)) = TreeBody::Leaf(3);
    *tree.get_mut_or_create(voxel::Bounds::new(9, 0, 16, 2)) = TreeBody::Leaf(4);
    *tree.get_mut_or_create(voxel::Bounds::new(9, 0, 16, 2)) = TreeBody::Leaf(5);

    assert_eq!(tree.get(voxel::Bounds::new(1, 1, 1, 0)), Some(&1));
    assert_eq!(tree.get(voxel::Bounds::new(8, -8, 4, 0)), Some(&2));
    assert_eq!(tree.get(voxel::Bounds::new(9, 0, 16, 2)), Some(&5));

    assert_eq!(tree.get(voxel::Bounds::new(2, 0, 4, 4)), None);
  }

  #[test]
  fn wrong_voxel_size_is_not_found() {
    let mut tree: VoxelTree<i32> = VoxelTree::new();
    *tree.get_mut_or_create(voxel::Bounds::new(4, 4, -4, 1)) = TreeBody::Leaf(1);
    assert_eq!(tree.get(voxel::Bounds::new(4, 4, -4, 0)), None);
    assert_eq!(tree.get(voxel::Bounds::new(4, 4, -4, 2)), None);
  }

  #[test]
  fn grow_is_transparent() {
    let mut tree: VoxelTree<i32> = VoxelTree::new();
    *tree.get_mut_or_create(voxel::Bounds::new(1, 1, 1, 0)) = TreeBody::Leaf(1);
    tree.grow_to_hold(voxel::Bounds::new(0, 0, 0, 1));
    tree.grow_to_hold(voxel::Bounds::new(0, 0, 0, 2));
    tree.grow_to_hold(voxel::Bounds::new(-32, 32, -128, 3));

    assert_eq!(tree.get(voxel::Bounds::new(1, 1, 1, 0)), Some(&1));
  }

  #[test]
  fn simple_cast_ray() {
    let mut tree: VoxelTree<i32> = VoxelTree::new();
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
    let mut tree: VoxelTree<i32> = VoxelTree::new();
    tree.grow_to_hold(voxel::Bounds::new(0, 0, 0, 30));
    bencher.iter(|| {
      *tree.get_mut_or_create(voxel::Bounds::new(0, 0, 0, 0)) = TreeBody::Leaf(0);
    });
    test::black_box(tree);
  }

  #[bench]
  fn bench_cast_ray(bencher: &mut test::Bencher) {
    let mut tree: VoxelTree<i32> = VoxelTree::new();
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
