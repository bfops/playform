use cgmath::{Point3};
use collision::{Aabb3};
use std::fmt::Debug;
use std::ptr;

pub const MIN_CELL_WIDTH: f32 = 0.1;

fn aabb_overlap(aabb1: &Aabb3<f32>, aabb2: &Aabb3<f32>) -> bool {
  true
  && aabb1.min.x < aabb2.max.x
  && aabb1.min.y < aabb2.max.y
  && aabb1.min.z < aabb2.max.z
  && aabb2.min.x < aabb1.max.x
  && aabb2.min.y < aabb1.max.y
  && aabb2.min.z < aabb1.max.z
}

fn contains(aabb1: &Aabb3<f32>, aabb2: &Aabb3<f32>) -> bool {
  true
  && aabb1.min.x <= aabb2.min.x
  && aabb1.min.y <= aabb2.min.y
  && aabb1.min.z <= aabb2.min.z
  && aabb2.max.x <= aabb1.max.x
  && aabb2.max.y <= aabb1.max.y
  && aabb2.max.z <= aabb1.max.z
}

fn length(bounds: &Aabb3<f32>, d: Dimension) -> f32 {
  get(d, &bounds.max) - get(d, &bounds.min)
}

fn middle(bounds: &Aabb3<f32>, d: Dimension) -> f32 {
  (get(d, &bounds.max) + get(d, &bounds.min)) / 2.0
}

fn get(d: Dimension, p: &Point3<f32>) -> f32 {
  match d {
    Dimension::X => p.x,
    Dimension::Y => p.y,
    Dimension::Z => p.z,
  }
}

fn set(d: Dimension, p: &mut Point3<f32>, v: f32) {
  match d {
    Dimension::X => p.x = v,
    Dimension::Y => p.y = v,
    Dimension::Z => p.z = v,
  }
}

fn split(mid: f32, d: Dimension, bounds: &Aabb3<f32>) -> (Option<Aabb3<f32>>, Option<Aabb3<f32>>) {
  if get(d, &bounds.max) <= mid {
    (Some(*bounds), None)
  } else if get(d, &bounds.min) >= mid {
    (None, Some(*bounds))
  } else {
    let (new_min, new_max) = {
      let (mut new_min, mut new_max) = (bounds.min, bounds.max);
      set(d, &mut new_min, mid);
      set(d, &mut new_max, mid);
      (new_min, new_max)
    };
    (
      Some(Aabb3::new(bounds.min, new_max)),
      Some(Aabb3::new(new_min, bounds.max))
    )
  }
}

#[derive(Copy, Clone)]
pub enum Dimension { X, Y, Z }

struct Branches<V> {
  low_tree: Box<Octree<V>>,
  high_tree: Box<Octree<V>>,
}

type LeafContents<V> = Vec<(Aabb3<f32>, V)>;

enum OctreeContents<V> {
  Leaf(LeafContents<V>),
  Branch(Branches<V>),
}

// TODO: allow inserting things with a "mobile" flag; don't subdivide those objects.
pub struct Octree<V> {
  parent: *mut Octree<V>,
  dimension: Dimension,
  bounds: Aabb3<f32>,
  contents: OctreeContents<V>,
}

unsafe impl<V: Send + 'static> Send for Octree<V> {}
#[allow(missing_docs, dead_code)]
/// Make sure `impl Send for Octree` is safe
fn impl_send_for_octree_is_safe<V: Send + 'static>() {
  fn assert_is_send<T: Send>() {}
  assert_is_send::<Dimension>();
  assert_is_send::<Aabb3<f32>>();
  assert_is_send::<OctreeContents<V>>();
}

// TODO: fix shaky octree outline insertion/removal conditions.

impl<V: Debug + Copy + Eq + PartialOrd> Octree<V> {
  pub fn new(bounds: &Aabb3<f32>) -> Octree<V> {
    Octree {
      parent: ptr::null_mut(),
      dimension: Dimension::X,
      bounds: *bounds,
      contents: OctreeContents::Leaf(Vec::new()),
    }
  }

  pub fn insert(&mut self, bounds: &Aabb3<f32>, v: V) {
    let this: *mut Octree<V> = self;
    assert!(contains(&self.bounds, &bounds));
    let contents = match self.contents {
      OctreeContents::Leaf(ref mut vs) => {
        vs.push((*bounds, v));

        let d = self.dimension;
        let avg_length =
          vs.iter().fold(
            0.0,
            |x, &(bounds, _)| x + length(&bounds, d)
          ) / (vs.len() as f32);

        let l = length(&self.bounds, self.dimension);
        let should_bisect_cell =
          l > MIN_CELL_WIDTH && avg_length < length(&self.bounds, self.dimension) / 2.0;
        if should_bisect_cell {
          let (low, high) =
            Octree::bisect(
              this,
              &self.bounds,
              self.dimension,
              vs
            );
          Some(OctreeContents::Branch(Branches {
            low_tree: Box::new(low),
            high_tree: Box::new(high),
          }))
        } else {
          None
        }
      },
      OctreeContents::Branch(ref mut b) => {
        // copied in remove()
        let (l, h) = split(middle(&self.bounds, self.dimension), self.dimension, bounds);
        l.map(|low_half| b.low_tree.insert(&low_half, v));
        h.map(|high_half| b.high_tree.insert(&high_half, v));
        None
      },
    };
    contents.map(|c| self.contents = c);
  }

  // Split a leaf into two subtrees.
  fn bisect(
    parent: *mut Octree<V>,
    bounds: &Aabb3<f32>,
    dimension: Dimension,
    vs: &LeafContents<V>
  ) -> (Octree<V>, Octree<V>) {
    let mid = middle(bounds, dimension);
    let (low_bounds, high_bounds) = split(mid, dimension, bounds);
    let low_bounds = low_bounds.unwrap();
    let high_bounds = high_bounds.unwrap();
    let new_d = match dimension {
        Dimension::X => Dimension::Y,
        Dimension::Y => Dimension::Z,
        Dimension::Z => Dimension::X,
      };

    let mut low = Octree {
      parent: parent,
      dimension: new_d,
      bounds: low_bounds,
      contents: OctreeContents::Leaf(Vec::new()),
    };
    let mut high = Octree {
      parent: parent,
      dimension: new_d,
      bounds: high_bounds,
      contents: OctreeContents::Leaf(Vec::new()),
    };

    for &(bounds, v) in vs.iter() {
      let (low_bounds, high_bounds) = split(mid, dimension, &bounds);
      low_bounds.map(|bs| low.insert(&bs, v));
      high_bounds.map(|bs| high.insert(&bs, v));
    }

    (low, high)
  }

  #[allow(dead_code)]
  fn on_ancestor<T, F>(&self, bounds: &Aabb3<f32>, mut f: F) -> T
    where F: FnMut(&Octree<V>) -> T
  {
    if contains(&self.bounds, bounds) {
      f(self)
    } else {
      unsafe {
        assert!(!self.parent.is_null());
        (*self.parent).on_ancestor(bounds, f)
      }
    }
  }

  fn on_mut_ancestor<T, F>(&mut self, bounds: &Aabb3<f32>, mut f: F) -> T
    where F: FnMut(&mut Octree<V>) -> T
  {
    if contains(&self.bounds, bounds) {
      f(self)
    } else {
      unsafe {
        assert!(!self.parent.is_null());
        (*self.parent).on_mut_ancestor(bounds, f)
      }
    }
  }

  // Find whether there are objects overlapping the object & bounds provided in
  // this/child trees. Uses equality comparison on V to ignore "same" objects.
  // Returns the value associated with the first object intersected.
  pub fn intersect(&self, bounds: &Aabb3<f32>, self_v: Option<V>) -> Option<(Aabb3<f32>, V)> {
    match self.contents {
      OctreeContents::Leaf(ref vs) => {
        vs.iter()
          .find(|&&(ref bs, ref v)| Some(*v) != self_v && aabb_overlap(bounds, bs))
          .map(|&(bounds, v)| (bounds, v))
      },
      OctreeContents::Branch(ref b) => {
        let mid = middle(&self.bounds, self.dimension);
        let (low_bounds, high_bounds) = split(mid, self.dimension, bounds);
        let high = |high_bounds| {
          match high_bounds {
            None => None,
            Some(bs) => b.high_tree.intersect(&bs, self_v),
          }
        };
        match low_bounds {
          None => high(high_bounds),
          Some(bs) =>
            match b.low_tree.intersect(&bs, self_v) {
              None => high(high_bounds),
              r => r,
            }
        }
      },
    }
  }

  // like insert, but before recursing downward, we recurse up the parents
  // until the bounds provided are inside the tree.
  fn insert_from(&mut self, bounds: &Aabb3<f32>, v: V) {
    self.on_mut_ancestor(bounds, |t| t.insert(bounds, v))
  }

  pub fn remove(&mut self, bounds: &Aabb3<f32>, v: V) {
    assert!(contains(&self.bounds, bounds));
    let collapse_contents = match self.contents {
      OctreeContents::Leaf(ref mut vs) => {
        match vs.iter().position(|&(_, ref x)| *x == v) {
          None => {
            panic!("{:?} was not found in the octree", v);
          },
          Some(i) => {
            vs.swap_remove(i);
          },
        };
        false
      },
      OctreeContents::Branch(ref mut bs) => {
        let (l, h) = split(middle(&self.bounds, self.dimension), self.dimension, bounds);
        l.map(|low_half| bs.low_tree.remove(&low_half, v));
        h.map(|high_half| bs.high_tree.remove(&high_half, v));
        bs.low_tree.is_empty() && bs.high_tree.is_empty()
      }
    };

    if collapse_contents {
      self.contents = OctreeContents::Leaf(Vec::new());
    }
  }

  pub fn is_empty(&self) -> bool {
    match self.contents {
      OctreeContents::Leaf(ref vs) => vs.is_empty(),
      _ => false,
    }
  }

  pub fn reinsert(&mut self, v: V, bounds: &Aabb3<f32>, new_bounds: &Aabb3<f32>) {
    self.remove(bounds, v);
    self.insert_from(new_bounds, v)
  }
}
