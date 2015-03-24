use std::mem;
use std::ops::Deref;

#[cfg(test)]
use test;

#[derive(Copy, FromPrimitive)]
pub enum Dimension {
  X = 0,
  Y = 1,
  Z = 2,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct VoxelBounds {
  pub x: i32,
  pub y: i32,
  pub z: i32,
  /// The log_2 of the voxel's scale, i.e. its size.
  pub lg_size: u8,
}

#[derive(Debug)]
pub struct VoxelTree<T> {
  /// The log_2 of the tree's size.
  lg_size: u8,
  contents: Branches<T>,
}

#[derive(Debug)]
#[repr(C)]
pub struct Branches<T> {
  // xyz ordering

  lll: TreeBody<T>,
  llh: TreeBody<T>,
  lhl: TreeBody<T>,
  lhh: TreeBody<T>,
  hll: TreeBody<T>,
  hlh: TreeBody<T>,
  hhl: TreeBody<T>,
  hhh: TreeBody<T>,
}

#[derive(Debug)]
pub enum TreeBody<T> {
  Empty,
  Leaf(T),
  Branch(Box<Branches<T>>),
}

impl VoxelBounds {
  pub fn new(x: i32, y: i32, z: i32, lg_size: u8) -> VoxelBounds {
    let ret =
      VoxelBounds {
        x: x,
        y: y,
        z: z,
        lg_size: lg_size,
      };
    assert!(ret.is_aligned());
    ret
  }

  pub fn is_aligned(&self) -> bool {
    let combined = self.x | self.y | self.z;
    let mask = (1 << self.lg_size) - 1;
    // Check no bits less significant than `lg_size` exist; i.e. alignment to `lg_size` is true.
    (combined & mask) == 0
  }

  pub fn is_within(&self, lg_size: u8) -> bool {
    assert!(self.is_aligned());

    let high = 1 << lg_size;
    let low = -high;

    if self.x <= low || self.y <= low || self.z <= low {
      return false
    }

    let scale = 1 << self.lg_size;
    let high_x = self.x + scale;
    let high_y = self.y + scale;
    let high_z = self.z + scale;

    high_x <= high && high_y <= high && high_z <= high
  }
}

#[allow(dead_code)]
impl<T> VoxelTree<T> {
  pub fn new() -> VoxelTree<T> {
    VoxelTree {
      lg_size: 0,
      contents: Branches::new(),
    }
  }

  #[inline(always)]
  fn get_branch<'a, ChooseBranch>(
    branches: &'a Branches<T>,
    mut choose_branch: ChooseBranch,
    x: i32, y: i32, z: i32,
  ) -> &'a TreeBody<T>
    where ChooseBranch: FnMut(i32) -> bool,
  {
    // TODO: Make this branch-free by constructing the bools into an offset.
    match (choose_branch(x), choose_branch(y), choose_branch(z)) {
      (false, false, false) => &branches.lll,
      (false, false, true ) => &branches.llh,
      (false, true , false) => &branches.lhl,
      (false, true , true ) => &branches.lhh,
      (true , false, false) => &branches.hll,
      (true , false, true ) => &branches.hlh,
      (true , true , false) => &branches.hhl,
      (true , true , true ) => &branches.hhh,
    }
  }

  #[inline(always)]
  fn get_branch_mut<'a, ChooseBranch>(
    branches: &'a mut Branches<T>,
    mut choose_branch: ChooseBranch,
    x: i32, y: i32, z: i32,
  ) -> &'a mut TreeBody<T>
    where ChooseBranch: FnMut(i32) -> bool,
  {
    // TODO: Make this branch-free by constructing the bools into an offset.
    match (choose_branch(x), choose_branch(y), choose_branch(z)) {
      (false, false, false) => &mut branches.lll,
      (false, false, true ) => &mut branches.llh,
      (false, true , false) => &mut branches.lhl,
      (false, true , true ) => &mut branches.lhh,
      (true , false, false) => &mut branches.hll,
      (true , false, true ) => &mut branches.hlh,
      (true , true , false) => &mut branches.hhl,
      (true , true , true ) => &mut branches.hhh,
    }
  }

  /// Ensure that this tree can hold the provided voxel.
  pub fn grow_to_hold(&mut self, voxel: VoxelBounds) {
    // Grow the octree until the voxel is inside it.
    while !voxel.is_within(self.lg_size) {
      // Double the bounds in every direction.
      self.lg_size += 1;

      // Pull out `self.contents` so we can move out of it.
      let contents = mem::replace(&mut self.contents, Branches::new());

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
          let mut branches = Branches::new();
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

  fn insert_step<'a>(
    branch: &'a mut TreeBody<T>,
  ) -> &'a mut Branches<T> {
    // "Step down" the tree.
    match *branch {
      // Branches; we can go straight to the branching logic.
      TreeBody::Branch(ref mut b) => b,

      // Otherwise, keep going, but we need to insert a voxel inside the
      // space occupied by the current branch.

      TreeBody::Empty => {
        // Replace this branch with 8 empty sub-branches - who's gonna notice?
        *branch = TreeBody::Branch(Box::new(Branches::new()));

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
        *branch = TreeBody::Branch(Box::new(Branches::new()));

        match *branch {
          TreeBody::Branch(ref mut b) => b,
          _ => unreachable!(),
        }
      },
    }
  }

  fn mut_find<'a, Step, E>(
    &'a mut self,
    voxel: VoxelBounds,
    mut step: Step,
  ) -> Result<&'a mut TreeBody<T>, E> where
    Step: FnMut(&'a mut TreeBody<T>) -> Result<&'a mut Branches<T>, E>,
  {
    // When we compare the voxel position to octree bounds to choose subtrees
    // for insertion, we'll be comparing voxel position to values of 2^n and
    // -2^n, so we can just use the position bits to branch directly.
    // This actually works for negative values too, without much wrestling:
    // we need to branch on the sign bit up front, but after that, two's
    // complement magic means the branching on bits works regardless of sign.

    let mask = (1 << self.lg_size) >> 1;

    // The voxel is aligned in a multiple of its size, which is some 2^k,
    // so we don't need to branch by anything smaller than the voxel size.
    debug_assert!(voxel.is_aligned());

    // Shift everything down by the voxel's lg_size, so we can just compare
    // the mask to 0 to know when we're done.
    let x = voxel.x >> voxel.lg_size;
    let y = voxel.y >> voxel.lg_size;
    let z = voxel.z >> voxel.lg_size;
    let mut mask = mask >> voxel.lg_size;

    let mut branches = &mut self.contents;

    macro_rules! iter(
      ($mask:expr, $step:block) => {{
        let branches_temp = branches;
        let branch = VoxelTree::get_branch_mut(branches_temp, $mask, x, y, z);

        $step;
        // We've reached the voxel.
        if mask == 0 {
          return Ok(branch)
        }

        branches = try!(step(branch));
      }}
    );

    iter!(|x| x >= 0, {});

    loop {
      iter!(
        |x| { (x & mask) != 0 },
        // Branch through half this size next time.
        { mask = mask >> 1; }
      );
    }
  }

  fn find<'a, Step, E>(
    &'a self,
    voxel: VoxelBounds,
    mut step: Step,
  ) -> Result<&'a TreeBody<T>, E> where
    Step: FnMut(&'a TreeBody<T>) -> Result<&'a Branches<T>, E>,
  {
    // When we compare the voxel position to octree bounds to choose subtrees
    // for insertion, we'll be comparing voxel position to values of 2^n and
    // -2^n, so we can just use the position bits to branch directly.
    // This actually works for negative values too, without much wrestling:
    // we need to branch on the sign bit up front, but after that, two's
    // complement magic means the branching on bits works regardless of sign.

    let mask = (1 << self.lg_size) >> 1;

    // The voxel is aligned in a multiple of its size, which is some 2^k,
    // so we don't need to branch by anything smaller than the voxel size.
    debug_assert!(voxel.is_aligned());

    // Shift everything down by the voxel's lg_size, so we can just compare
    // the mask to 0 to know when we're done.
    let x = voxel.x >> voxel.lg_size;
    let y = voxel.y >> voxel.lg_size;
    let z = voxel.z >> voxel.lg_size;
    let mut mask = mask >> voxel.lg_size;

    let mut branches = &self.contents;

    macro_rules! iter(
      ($mask:expr, $step:block) => {{
        let branches_temp = branches;
        let branch = VoxelTree::get_branch(branches_temp, $mask, x, y, z);

        $step;
        // We've reached the voxel.
        if mask == 0 {
          return Ok(branch)
        }

        branches = try!(step(branch));
      }}
    );

    iter!(|x| x >= 0, {});

    loop {
      iter!(
        |x| { (x & mask) != 0 },
        // Branch through half this size next time.
        { mask = mask >> 1; }
      );
    }
  }

  /// If the specified voxel exists, return it.
  /// Otherwise, create it as empty and return a reference to it.
  pub fn get_mut<'a>(&'a mut self, voxel: VoxelBounds) -> &'a mut TreeBody<T> {
    self.grow_to_hold(voxel);
    let branch: Result<_, ()> =
      self.mut_find(voxel, |branch| { Ok(VoxelTree::insert_step(branch)) });
    branch.unwrap()
  }

  pub fn get<'a>(&'a self, voxel: VoxelBounds) -> Option<&'a T> {
    if !voxel.is_within(self.lg_size) {
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
}

impl<T> Branches<T> {
  pub fn new() -> Branches<T> {
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
}

#[test]
fn simple_test() {
  let mut tree: VoxelTree<i32> = VoxelTree::new();
  *tree.get_mut(VoxelBounds::new(1, 1, 1, 0)) = TreeBody::Leaf(1);
  *tree.get_mut(VoxelBounds::new(8, -8, 4, 0)) = TreeBody::Leaf(2);
  *tree.get_mut(VoxelBounds::new(32, 0, 64, 4)) = TreeBody::Leaf(3);
  *tree.get_mut(VoxelBounds::new(36, 0, 64, 2)) = TreeBody::Leaf(4);
  *tree.get_mut(VoxelBounds::new(36, 0, 64, 2)) = TreeBody::Leaf(5);

  assert_eq!(tree.get(VoxelBounds::new(1, 1, 1, 0)), Some(&1));
  assert_eq!(tree.get(VoxelBounds::new(8, -8, 4, 0)), Some(&2));
  assert_eq!(tree.get(VoxelBounds::new(36, 0, 64, 2)), Some(&5));

  assert_eq!(tree.get(VoxelBounds::new(32, 0, 64, 4)), None);
}

#[test]
fn wrong_voxel_size_is_not_found() {
  let mut tree: VoxelTree<i32> = VoxelTree::new();
  *tree.get_mut(VoxelBounds::new(4, 4, -4, 1)) = TreeBody::Leaf(1);
  assert_eq!(tree.get(VoxelBounds::new(4, 4, -4, 0)), None);
  assert_eq!(tree.get(VoxelBounds::new(4, 4, -4, 2)), None);
}

#[test]
fn grow_is_transparent() {
  let mut tree: VoxelTree<i32> = VoxelTree::new();
  *tree.get_mut(VoxelBounds::new(1, 1, 1, 0)) = TreeBody::Leaf(1);
  tree.grow_to_hold(VoxelBounds::new(0, 0, 0, 1));
  tree.grow_to_hold(VoxelBounds::new(0, 0, 0, 2));
  tree.grow_to_hold(VoxelBounds::new(-32, 32, -128, 3));

  assert_eq!(tree.get(VoxelBounds::new(1, 1, 1, 0)), Some(&1));
}

#[should_panic]
#[test]
fn misaligned_bounds() {
  let v = VoxelBounds::new(2, 2, 1, 1);
  test::black_box(v);
}

#[bench]
fn simple_inserts(bencher: &mut test::Bencher) {
  bencher.iter(|| {
    let mut tree: VoxelTree<i32> = VoxelTree::new();
    tree.grow_to_hold(VoxelBounds::new(0, 0, 0, 30));

    for i in -10..10 {
      *tree.get_mut(VoxelBounds::new(i, i, i, 0)) = TreeBody::Leaf(0);
    }

    test::black_box(tree);
  });
}
