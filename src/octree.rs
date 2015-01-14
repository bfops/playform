use common::*;
use nalgebra::Pnt3;
use nalgebra::partial_lt;
use ncollide::bounding_volume::{AABB, AABB3};
use ncollide::bounding_volume::BoundingVolume;
use ncollide::ray::{Ray3, LocalRayCast};
use std::cmp::Ordering::{Greater, Less, Equal};
use std::num::NumCast;
use std::ptr;

pub const MIN_CELL_WIDTH: f32 = 0.1;

fn aabb_overlap(aabb1: &AABB3<f32>, aabb2: &AABB3<f32>) -> bool {
  partial_lt(aabb1.mins(), aabb2.maxs()) &&
  partial_lt(aabb2.mins(), aabb1.maxs())
}

fn length(bounds: &AABB3<f32>, d: Dimension) -> f32 {
  get(d, bounds.maxs()) - get(d, bounds.mins())
}

fn middle(bounds: &AABB3<f32>, d: Dimension) -> f32 {
  (get(d, bounds.maxs()) + get(d, bounds.mins())) / 2.0
}

fn get(d: Dimension, p: &Pnt3<f32>) -> f32 {
  match d {
    Dimension::X => p.x,
    Dimension::Y => p.y,
    Dimension::Z => p.z,
  }
}

fn set(d: Dimension, p: &mut Pnt3<f32>, v: f32) {
  match d {
    Dimension::X => p.x = v,
    Dimension::Y => p.y = v,
    Dimension::Z => p.z = v,
  }
}

fn split(mid: f32, d: Dimension, bounds: AABB3<f32>) -> (Option<AABB3<f32>>, Option<AABB3<f32>>) {
  if get(d, bounds.maxs()) <= mid {
    (Some(bounds), None)
  } else if get(d, bounds.mins()) >= mid {
    (None, Some(bounds))
  } else {
    let (new_min, new_max) = {
      let (mut new_min, mut new_max) = (bounds.mins().clone(), bounds.maxs().clone());
      set(d, &mut new_min, mid.clone());
      set(d, &mut new_max, mid);
      (new_min, new_max)
    };
    (
      Some(AABB::new(*bounds.mins(), new_max)),
      Some(AABB::new(new_min, *bounds.maxs()))
    )
  }
}

#[derive(Copy)]
pub enum Dimension { X, Y, Z }

struct Branches<V> {
  low_tree: Box<Octree<V>>,
  high_tree: Box<Octree<V>>,
}

type LeafContents<V> = Vec<(AABB3<f32>, V)>;

enum OctreeContents<V> {
  Leaf(LeafContents<V>),
  Branch(Branches<V>),
}

// TODO: allow inserting things with a "mobile" flag; don't subdivide those objects.
pub struct Octree<V> {
  parent: *mut Octree<V>,
  dimension: Dimension,
  bounds: AABB3<f32>,
  contents: OctreeContents<V>,
}

// TODO: fix shaky octree outline insertion/removal conditions.

impl<V: Copy + Eq + PartialOrd> Octree<V> {
  pub fn new(bounds: &AABB3<f32>) -> Octree<V> {
    Octree {
      parent: ptr::null_mut(),
      dimension: Dimension::X,
      bounds: bounds.clone(),
      contents: OctreeContents::Leaf(Vec::new()),
    }
  }

  pub fn insert(&mut self, bounds: AABB3<f32>, v: V) {
    assert!(self.bounds.contains(&bounds));
    let contents = match self.contents {
      OctreeContents::Leaf(ref mut vs) => {
        vs.push((bounds, v));

        let d = self.dimension;
        let avg_length =
          vs.iter().fold(
            0.0,
            |x, &(ref bounds, _)| x + length(bounds, d)
          ) / NumCast::from(vs.len()).unwrap();

        let l = length(&self.bounds, self.dimension);
        let should_bisect_cell =
          l > MIN_CELL_WIDTH && avg_length < length(&self.bounds, self.dimension) / 2.0;
        if should_bisect_cell {
          let (low, high) =
            Octree::bisect(
              self,
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
        l.map(|low_half| b.low_tree.insert(low_half, v));
        h.map(|high_half| b.high_tree.insert(high_half, v));
        None
      },
    };
    contents.map(|c| self.contents = c);
  }

  // Split a leaf into two subtrees.
  fn bisect(
    parent: *mut Octree<V>,
    bounds: &AABB3<f32>,
    dimension: Dimension,
    vs: &LeafContents<V>
  ) -> (Octree<V>, Octree<V>) {
    let mid = middle(bounds, dimension);
    let (low_bounds, high_bounds) = split(mid, dimension, bounds.clone());
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
      bounds: low_bounds.clone(),
      contents: OctreeContents::Leaf(Vec::new()),
    };
    let mut high = Octree {
      parent: parent,
      dimension: new_d,
      bounds: high_bounds.clone(),
      contents: OctreeContents::Leaf(Vec::new()),
    };

    for &(ref bounds, v) in vs.iter() {
      let (low_bounds, high_bounds) = split(mid, dimension, bounds.clone());
      low_bounds.map(|bs| low.insert(bs, v));
      high_bounds.map(|bs| high.insert(bs, v));
    }

    (low, high)
  }

  #[allow(dead_code)]
  fn on_ancestor<T, F>(&self, bounds: &AABB3<f32>, mut f: F) -> T
    where F: FnMut(&Octree<V>) -> T
  {
    if self.bounds.contains(bounds) {
      f(self)
    } else {
      unsafe {
        assert!(!self.parent.is_null());
        (*self.parent).on_ancestor(bounds, f)
      }
    }
  }

  fn on_mut_ancestor<T, F>(&mut self, bounds: &AABB3<f32>, mut f: F) -> T
    where F: FnMut(&mut Octree<V>) -> T
  {
    if self.bounds.contains(bounds) {
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
  pub fn intersect(&self, bounds: &AABB3<f32>, self_v: Option<V>) -> Option<(AABB3<f32>, V)> {
    match self.contents {
      OctreeContents::Leaf(ref vs) => {
        vs.iter()
          .find(|&&(ref bs, ref v)| Some(*v) != self_v && aabb_overlap(bounds, bs))
          .map(|&(ref bounds, v)| (bounds.clone(), v))
      },
      OctreeContents::Branch(ref b) => {
        let mid = middle(&self.bounds, self.dimension);
        let (low_bounds, high_bounds) = split(mid, self.dimension, bounds.clone());
        let high = |&: high_bounds| {
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
  fn insert_from(&mut self, bounds: AABB3<f32>, v: V) {
    self.on_mut_ancestor(&bounds, |t| t.insert(bounds.clone(), v))
  }

  pub fn remove(&mut self, bounds: &AABB3<f32>, v: V) {
    assert!(self.bounds.contains(bounds));
    let collapse_contents = match self.contents {
      OctreeContents::Leaf(ref mut vs) => {
        let i = vs.iter().position(|&(_, ref x)| *x == v).unwrap();
        vs.swap_remove(i);
        false
      },
      OctreeContents::Branch(ref mut bs) => {
        let (l, h) = split(middle(&self.bounds, self.dimension), self.dimension, bounds.clone());
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

  pub fn reinsert(&mut self, v: V, bounds: &AABB3<f32>, new_bounds: AABB3<f32>) {
    self.remove(bounds, v);
    self.insert_from(new_bounds, v)
  }

  #[allow(dead_code)]
  pub fn cast_ray(&self, ray: &Ray3<f32>, self_v: V) -> Vec<V> {
    match self.contents {
      OctreeContents::Leaf(ref vs) => {
        // find the time of intersection (TOI) of the ray with each object in
        // this leaf; filter out the objects it doesn't intersect at all. Then
        // find the object with the lowest TOI.
        partial_min_by(
          vs.iter().filter_map(|&(ref bounds, v)| {
              if v == self_v {
                None
              } else {
                bounds.toi_with_ray(ray, true).map(|x| (x, v))
              }
            }
          ),
          |(toi, _)| toi
        )
        .into_iter().map(|(_, v)| v).collect()
      },
      OctreeContents::Branch(ref bs) => {
        let mut trees: Vec<(f32, &Box<Octree<V>>)> = Vec::new();
        let ref l = bs.low_tree;
        let ref h = bs.high_tree;
        for &t in [l, h].iter() {
          (**t).bounds.toi_with_ray(ray, true).map(|toi| trees.push((toi, t)));
        }
        trees.sort_by(|&(t1, _), &(t2, _)| {
          if t1 < t2 {
            Less
          } else if t1 > t2 {
            Greater
          } else {
            Equal
          }
        });
        for &(_, t) in trees.iter() {
          let r = t.cast_ray(ray, self_v);
          if !r.is_empty() {
            return r;
          }
        }
        Vec::new()
      }
    }
  }
}
