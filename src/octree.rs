use common::*;
use loader::{Loader,Load,Unload};
use nalgebra::Pnt3;
use ncollide::bounding_volume::{AABB, AABB3};
use ncollide::bounding_volume::BoundingVolume;
use ncollide::ray::{Ray3, LocalRayCast};
use std::cell::RefCell;
use std::collections::{HashMap,HashSet};
use std::hash::Hash;
use std::num::NumCast;
use std::ptr::RawPtr;
use std::rc::Rc;
use glw::gl_buffer::*;
use glw::gl_context::GLContext;
use glw::shader::Shader;
use glw::vertex;
use glw::vertex::ColoredVertex;

fn length(bounds: &AABB3, d: Dimension) -> f32 {
  get(d, bounds.maxs()) - get(d, bounds.mins())
}

fn middle(bounds: &AABB3, d: Dimension) -> f32 {
  (get(d, bounds.maxs()) + get(d, bounds.mins())) / 2.0
}

fn get(d: Dimension, p: &Pnt3<f32>) -> f32 {
  match d {
    X => p.x,
    Y => p.y,
    Z => p.z,
  }
}

fn set(d: Dimension, p: &mut Pnt3<f32>, v: f32) {
  match d {
    X => p.x = v,
    Y => p.y = v,
    Z => p.z = v,
  }
}

fn split(mid: f32, d: Dimension, bounds: AABB3) -> (Option<AABB3>, Option<AABB3>) {
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

pub enum Dimension { X, Y, Z }

#[deriving(Copy, Clone, PartialEq, Eq, Hash, Show)]
pub struct OctreeId(uint);

impl Add<uint, OctreeId> for OctreeId {
  fn add(&self, rhs: &uint) -> OctreeId {
    let OctreeId(id) = *self;
    OctreeId(id + *rhs)
  }
}

pub struct OctreeBuffers<V> {
  entry_to_index: HashMap<OctreeId, uint>,
  index_to_entry: Vec<OctreeId>,

  outlines: GLArray<ColoredVertex>,
}

impl<V> OctreeBuffers<V> {
  pub unsafe fn new(
      gl: &GLContext,
      shader_program: &Rc<RefCell<Shader>>
  ) -> OctreeBuffers<V> {
    OctreeBuffers {
      entry_to_index: HashMap::new(),
      index_to_entry: Vec::new(),

      outlines: GLArray::new(
        gl,
        shader_program.clone(),
        [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
        ],
        Lines,
        GLBuffer::new(10 * MAX_WORLD_SIZE),
      ),
    }
  }

  pub fn push(
    &mut self,
    entry: OctreeId,
    outlines: &[ColoredVertex]
  ) {
    assert!(!self.entry_to_index.contains_key(&entry));
    self.entry_to_index.insert(entry, self.index_to_entry.len());
    self.index_to_entry.push(entry);

    self.outlines.push(outlines);
  }

  pub fn swap_remove(&mut self, entry: OctreeId) {
    let &idx = self.entry_to_index.find(&entry).unwrap();
    let swapped_id = self.index_to_entry[self.index_to_entry.len() - 1];
    self.index_to_entry.swap_remove(idx).unwrap();
    self.entry_to_index.remove(&entry);
    self.outlines.swap_remove(idx * LINE_VERTICES_PER_BOX, LINE_VERTICES_PER_BOX);
    if entry != swapped_id {
      self.entry_to_index.insert(swapped_id, idx);
      assert!(self.index_to_entry[idx] == swapped_id);
    }
  }

  pub fn draw(&self, gl: &GLContext) {
    self.outlines.draw(gl);
  }
}

struct Branches<V> {
  low_tree: Box<Octree<V>>,
  high_tree: Box<Octree<V>>,
}

type LeafContents<V> = Vec<(AABB3, V)>;

enum OctreeContents<V> {
  Leaf(LeafContents<V>),
  Branch(Branches<V>),
}

static mut next_id: OctreeId = OctreeId(0);

pub type OctreeLoader = Loader<(OctreeId, AABB3), OctreeId>;

// TODO: allow inserting things with a "mobile" flag; don't subdivide those objects.
pub struct Octree<V> {
  parent: *mut Octree<V>,
  dimension: Dimension,
  bounds: AABB3,
  contents: OctreeContents<V>,

  // for rendering
  id: OctreeId,
  loader: Rc<RefCell<OctreeLoader>>,
}

// TODO: fix shaky octree outline insertion/removal conditions.

impl<V: Copy + Eq + PartialOrd + Hash> Octree<V> {
  pub fn new(loader: Rc<RefCell<OctreeLoader>>, bounds: &AABB3) -> Octree<V> {
    Octree {
      parent: RawPtr::null(),
      dimension: X,
      bounds: bounds.clone(),
      contents: Leaf(Vec::new()),
      id: Octree::<V>::alloc_id(),
      loader: loader,
    }
  }

  /// Create a new unique OctreeId.
  fn alloc_id() -> OctreeId {
    unsafe {
      let id = next_id;
      next_id = next_id + 1;
      id
    }
  }

  pub fn insert(&mut self, bounds: AABB3, v: V) {
    assert!(self.bounds.contains(&bounds));
    let contents = match self.contents {
      Leaf(ref mut vs) => {
        if vs.is_empty() {
          self.loader.borrow_mut().push(Load((self.id, self.bounds.clone())));
        }

        vs.push((bounds, v));

        let d = self.dimension;
        let avg_length =
          vs.iter().fold(
            0.0,
            |x, &(bounds, _)| x + length(&bounds, d)
          ) / NumCast::from(vs.len()).unwrap();

        if avg_length < length(&self.bounds, self.dimension) / 2.0 {
          self.loader.borrow_mut().push(Unload(self.id));

          let (low, high) =
            Octree::bisect(
              self,
              &self.loader,
              &self.bounds,
              self.dimension,
              vs
            );
          Some(Branch(Branches { low_tree: box low, high_tree: box high }))
        } else {
          None
        }
      },
      Branch(ref mut b) => {
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
      loader: &Rc<RefCell<OctreeLoader>>,
      bounds: &AABB3,
      dimension: Dimension,
      vs: &LeafContents<V>
  ) -> (Octree<V>, Octree<V>) {
    let mid = middle(bounds, dimension);
    let (low_bounds, high_bounds) = split(mid, dimension, bounds.clone());
    let low_bounds = low_bounds.unwrap();
    let high_bounds = high_bounds.unwrap();
    let new_d = match dimension {
        X => Y,
        Y => Z,
        Z => X,
      };

    let mut low = Octree {
      parent: parent,
      dimension: new_d,
      bounds: low_bounds.clone(),
      contents: Leaf(Vec::new()),
      id: Octree::<V>::alloc_id(),
      loader: loader.clone(),
    };
    let mut high = Octree {
      parent: parent,
      dimension: new_d,
      bounds: high_bounds.clone(),
      contents: Leaf(Vec::new()),
      id: Octree::<V>::alloc_id(),
      loader: loader.clone(),
    };

    for &(bounds, v) in vs.iter() {
      let (low_bounds, high_bounds) = split(mid, dimension, bounds);
      low_bounds.map(|bs| low.insert(bs, v));
      high_bounds.map(|bs| high.insert(bs, v));
    }

    (low, high)
  }

  fn on_ancestor<T>(&self, bounds: &AABB3, f: |&Octree<V>| -> T) -> T {
    if self.bounds.contains(bounds) {
      f(self)
    } else {
      unsafe {
        assert!(self.parent.is_not_null());
        (*self.parent).on_ancestor(bounds, f)
      }
    }
  }

  fn on_mut_ancestor<T>(&mut self, bounds: &AABB3, f: |&mut Octree<V>| -> T) -> T {
    if self.bounds.contains(bounds) {
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
  // Returns the value associated with the first object intersected.
  pub fn intersect(&self, bounds: &AABB3, self_v: Option<V>) -> Option<(AABB3, V)> {
    match self.contents {
      Leaf(ref vs) => {
        vs.iter()
          .find(|&&(bs, ref v)| Some(*v) != self_v && bounds.intersects(&bs))
          .map(|&(bounds, v)| (bounds.clone(), v))
      },
      Branch(ref b) => {
        let mid = middle(&self.bounds, self.dimension);
        let (low_bounds, high_bounds) = split(mid, self.dimension, *bounds);
        let high = || {
          match high_bounds {
            None => None,
            Some(bs) => b.high_tree.intersect(&bs, self_v),
          }
        };
        match low_bounds {
          None => high(),
          Some(bs) =>
            match b.low_tree.intersect(&bs, self_v) {
              None => high(),
              r => r,
            }
        }
      },
    }
  }

  // Find details of objects overlapping the object & bounds provided in this
  // and child trees. Uses equality comparison on V to ignore "same" objects.
  // Only finds intersects in this and child trees.
  pub fn intersect_details(&self, bounds: &AABB3, self_v: V) -> HashSet<V> {
    match self.contents {
      Leaf(ref vs) => {
        let mut r = HashSet::new();
        for &(bs, v) in vs.iter() {
          if v != self_v {
            if bounds.intersects(&bs) {
              r.insert(v);
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
          for v in b.low_tree.intersect_details(&bs, self_v).iter() {
            r.insert(*v);
          }
        });
        high_bounds.map(|bs| {
          for v in b.high_tree.intersect_details(&bs, self_v).iter() {
            r.insert(*v);
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
  pub fn intersect_details_from(&self, bounds: &AABB3, self_v: V) -> HashSet<V> {
    self.on_ancestor(bounds, |t| t.intersect_details(bounds, self_v))
  }

  // like insert, but before recursing downward, we recurse up the parents
  // until the bounds provided are inside the tree.
  fn insert_from(&mut self, bounds: AABB3, v: V) {
    self.on_mut_ancestor(&bounds, |t| t.insert(bounds.clone(), v))
  }

  pub fn remove(&mut self, v: V, bounds: &AABB3) {
    assert!(self.bounds.contains(bounds));
    let collapse_contents = match self.contents {
      Leaf(ref mut vs) => {
        let i = vs.iter().position(|&(_, ref x)| *x == v).unwrap();
        vs.swap_remove(i);
        if vs.is_empty() {
          self.loader.borrow_mut().push(Unload(self.id));
          false
        } else {
          false
        }
      },
      Branch(ref mut bs) => {
        let (l, h) = split(middle(&self.bounds, self.dimension), self.dimension, *bounds);
        l.map(|low_half| bs.low_tree.remove(v, &low_half));
        h.map(|high_half| bs.high_tree.remove(v, &high_half));
        bs.low_tree.is_empty() && bs.high_tree.is_empty()
      }
    };

    if collapse_contents {
      self.contents = Leaf(Vec::new());
    }
  }

  pub fn is_empty(&self) -> bool {
    match self.contents {
      Leaf(ref vs) => vs.is_empty(),
      _ => false,
    }
  }

  pub fn reinsert(&mut self, v: V, bounds: &AABB3, new_bounds: AABB3) {
    self.remove(v, bounds);
    self.insert_from(new_bounds, v)
  }

  pub fn cast_ray(&self, ray: &Ray3, self_v: V) -> Vec<V> {
    match self.contents {
      Leaf(ref vs) => {
        // find the time of intersection (TOI) of the ray with each object in
        // this leaf; filter out the objects it doesn't intersect at all. Then
        // find the object with the lowest TOI.
        partial_min_by(
          vs.iter().filter_map(|&(bounds, v)| {
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
      Branch(ref bs) => {
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
