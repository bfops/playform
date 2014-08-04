use cgmath::aabb::Aabb3;
use cgmath::point::{Point, Point3};
use std::collections::HashSet;
use std::hash::Hash;
use std::mem;
use std::ptr::RawPtr;

type F = f32;

fn less(p1: &Point3<F>, p2: &Point3<F>) -> bool {
  p1.x < p2.x &&
  p1.y < p2.y &&
  p1.z < p2.z
}

fn lequal(p1: &Point3<F>, p2: &Point3<F>) -> bool {
  p1.x <= p2.x &&
  p1.y <= p2.y &&
  p1.z <= p2.z
}

fn intersect(b1: &Aabb3<F>, b2: &Aabb3<F>) -> bool {
  let r = less(&b1.min, &b2.max) && less(&b2.min, &b1.max);
  r
}

fn contains(b1: &Aabb3<F>, b2: &Aabb3<F>) -> bool {
  lequal(&b1.min, &b2.min) && lequal(&b2.min, &b2.max)
}

fn length(bounds: &Aabb3<F>, d: Dimension) -> F {
  get(d, &bounds.max) - get(d, &bounds.min)
}

fn middle(bounds: &Aabb3<F>, d: Dimension) -> F {
  (get(d, &bounds.max) + get(d, &bounds.min)) / 2.0
}

fn get(d: Dimension, p: &Point3<F>) -> F {
  match d {
    X => p.x.clone(),
    Y => p.y.clone(),
    Z => p.z.clone(),
  }
}

fn set<F>(d: Dimension, p: &mut Point3<F>, v: F) {
  match d {
    X => p.x = v,
    Y => p.y = v,
    Z => p.z = v,
  }
}

fn split(mid: F, d: Dimension, bounds: Aabb3<F>) -> (Option<Aabb3<F>>, Option<Aabb3<F>>) {
  if get(d, &bounds.max) <= mid {
    (Some(bounds), None)
  } else if get(d, &bounds.min) >= mid {
    (None, Some(bounds))
  } else {
    let (new_min, new_max) = {
      let (mut new_min, mut new_max) = (bounds.min.clone(), bounds.max.clone());
      set(d, &mut new_min, mid.clone());
      set(d, &mut new_max, mid);
      (new_min, new_max)
    };
    (Some(Aabb3 { min: bounds.min, max: new_max }),
      Some(Aabb3 { min: new_min, max: bounds.max }))
  }
}

pub enum Dimension { X, Y, Z }

struct Branches<V> {
  low_tree: Box<Octree<V>>,
  high_tree: Box<Octree<V>>,
}

type LeafContents<V> = Vec<(Aabb3<F>, V)>;

enum OctreeContents<V> {
  Empty,
  Leaf(LeafContents<V>),
  Branch(Branches<V>),
}

pub struct Octree<V> {
  parent: *mut Octree<V>,
  dimension: Dimension,
  bounds: Aabb3<F>,
  contents: OctreeContents<V>,
}

impl<V: Clone + Eq + Hash> Octree<V> {
  pub fn new(bounds: &Aabb3<F>) -> Octree<V> {
    Octree {
      parent: RawPtr::null(),
      dimension: X,
      bounds: bounds.clone(),
      contents: Empty,
    }
  }

  pub fn insert(&mut self, bounds: Aabb3<F>, v: V) -> *mut Octree<V> {
    assert!(contains(&self.bounds, &bounds));
    let t: Option<*mut Octree<V>> =
      match self.contents {
        Empty => {
          let vs = Vec::from_elem(1, (bounds.clone(), v.clone()));
          // if the object mostly fills the leaf, don't bother subsecting space more.
          if length(&bounds, self.dimension) * (2.0 as f32) < length(&self.bounds, self.dimension) {
            let mid = middle(&self.bounds, self.dimension);

            let d = self.dimension;
            let (low, high) = self.bisect(mid);
            let (mut low, mut high) = (box low, box high);
            unsafe {
              let (low_bounds, high_bounds) = split(mid.clone(), d, bounds);
              let mut spans_multiple = false;
              let mut parent = None;
              low_bounds.map(
                |bs| {
                  low.insert(bs, v.clone());
                  spans_multiple = true;
                  parent = Some(mem::transmute(&mut *low));
                }
              );
              high_bounds.map(
                |bs| {
                  high.insert(bs, v.clone());
                  spans_multiple = true;
                  parent = Some(mem::transmute(&mut *high));
                }
              );

              self.contents = Branch(Branches { low_tree: low, high_tree: high });

              if spans_multiple {
                None
              } else {
                parent
              }
            }
          } else {
            self.contents = Leaf(vs);
            None
          }
        },
        Leaf(ref mut vs) => {
          vs.push((bounds, v));
          None
        },
        Branch(ref mut b) => {
          // copied in remove()
          let (l, h) = split(middle(&self.bounds, self.dimension), self.dimension, bounds);
          l.map(|low_half| { b.low_tree.insert(low_half, v.clone()); });
          h.map(|high_half| { b.high_tree.insert(high_half, v.clone()); });
          None
        },
      };
    unsafe {
      t.map_or(mem::transmute(self), |x| { mem::transmute(x) })
    }
  }

  // Split this tree into two sub-trees around a given point,
  fn bisect(&mut self, mid: F) -> (Octree<V>, Octree<V>) {
    let (low_bounds, high_bounds) = split(mid.clone(), self.dimension, self.bounds.clone());
    let low_bounds = low_bounds.expect("world bounds couldn't split on middle");
    let high_bounds = high_bounds.expect("world bounds couldn't split on middle");
    let new_d = match self.dimension { 
        X => Y,
        Y => Z,
        Z => X,
      };

    (
      Octree {
        parent: self,
        dimension: new_d,
        bounds: low_bounds.clone(),
        contents: Empty,
      },
      Octree {
        parent: self,
        dimension: new_d,
        bounds: high_bounds.clone(),
        contents: Empty
      }
    )
  }

  fn on_ancestor<T>(&self, bounds: &Aabb3<F>, f: |&Octree<V>| -> T) -> T {
    if contains(&self.bounds, bounds) {
      f(self)
    } else {
      unsafe {
        assert!(self.parent.is_not_null());
        (*self.parent).on_ancestor(bounds, f)
      }
    }
  }

  fn on_mut_ancestor<T>(&mut self, bounds: &Aabb3<F>, f: |&mut Octree<V>| -> T) -> T {
    if contains(&self.bounds, bounds) {
      f(self)
    } else {
      unsafe {
        assert!(self.parent.is_not_null());
        (*self.parent).on_mut_ancestor(bounds, f)
      }
    }
  }

  // Find whether there are objects overlapping the object & bounds provided in
  // this/child trees. Uses equality comparison on V to ignore "same" objects.
  pub fn intersect(&self, bounds: &Aabb3<F>, self_v: V) -> bool {
    match self.contents {
      Empty => false,
      Leaf(ref vs) => vs.iter().any(|&(bs, ref v)| { *v != self_v && intersect(bounds, &bs) }),
      Branch(ref b) => {
        let mid = middle(&self.bounds, self.dimension);
        let (low_bounds, high_bounds) = split(mid, self.dimension, *bounds);
        low_bounds.map_or(false, |bs| { b.low_tree.intersect(&bs, self_v.clone()) }) ||
        high_bounds.map_or(false, |bs| { b.high_tree.intersect(&bs, self_v.clone()) })
      }
    }
  }

  // Find details of objects overlapping the object & bounds provided in this
  // and child trees. Uses equality comparison on V to ignore "same" objects.
  // Only finds intersects in this and child trees.
  pub fn intersect_details(&self, bounds: &Aabb3<F>, self_v: V) -> HashSet<V> {
    match self.contents {
      Empty => HashSet::new(),
      Leaf(ref vs) => {
        let mut r = HashSet::new();
        for &(bs, ref v) in vs.iter() {
          if *v != self_v {
            if intersect(bounds, &bs) {
              r.insert(v.clone());
            }
          }
        }
        r
      },
      Branch(ref b) => {
        let mut r = HashSet::new();
        let mid = middle(&self.bounds, self.dimension);
        let (low_bounds, high_bounds) = split(mid, self.dimension, *bounds);
        low_bounds.map(|bs| {
          for ref v in b.low_tree.intersect_details(&bs, self_v.clone()).iter() {
            r.insert((*v).clone());
          }
        });
        high_bounds.map(|bs| {
          for ref v in b.high_tree.intersect_details(&bs, self_v.clone()).iter() {
            r.insert((*v).clone());
          }
        });
        r
      }
    }
  }

  #[allow(dead_code)]
  // Find the details of objects overlapping the object & bounds provided in
  // this tree, any children, or any relatives, starting the search from the
  // current tree. Uses equality comparison on V to ignore "same" objects.
  pub fn intersect_details_from(&self, bounds: &Aabb3<F>, self_v: V) -> HashSet<V> {
    self.on_ancestor(bounds, |t| { t.intersect_details(bounds, self_v.clone()) })
  }

  // like insert, but before recursing downward, we recurse up the parents
  // until the bounds provided are inside the tree.
  fn insert_from(&mut self, bounds: Aabb3<F>, v: V) -> *mut Octree<V> {
    self.on_mut_ancestor(&bounds, |t| { t.insert(bounds.clone(), v.clone()) })
  }

  pub fn remove(&mut self, v: V, bounds: &Aabb3<F>) {
    assert!(contains(&self.bounds, bounds));
    // copied largely from insert()
    match self.contents {
      Empty => {
        fail!("Could not Octree::remove(&Empty)");
      },
      Leaf(ref mut vs) => {
        let i = vs.iter().position(|&(_, ref x)| { *x == v }).expect("could not Octree::remove()");
        vs.swap_remove(i);
      },
      Branch(ref mut bs) => {
        let (l, h) = split(middle(&self.bounds, self.dimension), self.dimension, *bounds);
        l.map(|low_half| { bs.low_tree.remove(v.clone(), &low_half); });
        h.map(|high_half| { bs.high_tree.remove(v.clone(), &high_half); });
      }
    }
  }

  pub fn move(&mut self, v: V, bounds: &Aabb3<F>, new_bounds: Aabb3<F>) -> *mut Octree<V> {
    self.remove(v.clone(), bounds);
    self.insert_from(new_bounds, v)
  }
}
