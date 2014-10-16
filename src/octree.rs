use common::*;
use loader::{Loader,Load,Unload};
use nalgebra::Pnt3;
use ncollide::bounding_volume::aabb::AABB;
use ncollide::bounding_volume::BoundingVolume;
use ncollide::ray::{Ray, RayCast};
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

type F = f32;

fn length(bounds: &AABB, d: Dimension) -> F {
  get(d, bounds.maxs()) - get(d, bounds.mins())
}

fn middle(bounds: &AABB, d: Dimension) -> F {
  (get(d, bounds.maxs()) + get(d, bounds.mins())) / 2.0
}

fn get(d: Dimension, p: &Pnt3<F>) -> F {
  match d {
    X => p.x,
    Y => p.y,
    Z => p.z,
  }
}

fn set<F>(d: Dimension, p: &mut Pnt3<F>, v: F) {
  match d {
    X => p.x = v,
    Y => p.y = v,
    Z => p.z = v,
  }
}

fn split(mid: F, d: Dimension, bounds: AABB) -> (Option<AABB>, Option<AABB>) {
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

  outlines: GLSliceBuffer<ColoredVertex>,
}

impl<V> OctreeBuffers<V> {
  pub unsafe fn new(
      gl: &GLContext,
      shader_program: &Rc<RefCell<Shader>>
  ) -> OctreeBuffers<V> {
    OctreeBuffers {
      entry_to_index: HashMap::new(),
      index_to_entry: Vec::new(),

      outlines: GLSliceBuffer::new(
        gl,
        shader_program.clone(),
        [ vertex::AttribData { name: "position", size: 3, unit: vertex::Float },
          vertex::AttribData { name: "in_color", size: 4, unit: vertex::Float },
        ],
        LINE_VERTICES_PER_BOX,
        1 << 18,
        Lines
      ),
    }
  }

  pub fn push(
    &mut self,
    gl: &GLContext,
    entry: OctreeId,
    outlines: &[ColoredVertex]
  ) {
    assert!(!self.entry_to_index.contains_key(&entry));
    self.entry_to_index.insert(entry, self.index_to_entry.len());
    self.index_to_entry.push(entry);

    self.outlines.push(gl, outlines);
  }

  pub fn swap_remove(&mut self, gl: &GLContext, entry: OctreeId) {
    let &idx = self.entry_to_index.find(&entry).unwrap();
    let swapped_id = self.index_to_entry[self.index_to_entry.len() - 1];
    self.index_to_entry.swap_remove(idx).unwrap();
    self.entry_to_index.remove(&entry);
    self.outlines.swap_remove(gl, idx);
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

type LeafContents<V> = Vec<(AABB, V)>;

enum OctreeContents<V> {
  Leaf(LeafContents<V>),
  Branch(Branches<V>),
}

static mut next_id: OctreeId = OctreeId(0);

pub type OctreeLoader = Loader<(OctreeId, AABB), OctreeId>;

// TODO: allow inserting things with a "mobile" flag; don't subdivide those objects.
pub struct Octree<V> {
  parent: *mut Octree<V>,
  dimension: Dimension,
  bounds: AABB,
  contents: OctreeContents<V>,

  // for rendering
  id: OctreeId,
  loader: Rc<RefCell<OctreeLoader>>,
}

// TODO: fix shaky octree outline insertion/removal conditions.

impl<V: Copy + Eq + PartialOrd + Hash> Octree<V> {
  pub fn new(loader: Rc<RefCell<OctreeLoader>>, bounds: &AABB) -> Octree<V> {
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

  pub fn insert(&mut self, bounds: AABB, v: V) {
    assert!(self.bounds.contains(&bounds));
    let contents = match self.contents {
      Leaf(ref mut vs) => {
        if vs.is_empty() {
          self.loader.deref().borrow_mut().deref_mut().push(Load((self.id, self.bounds.clone())));
        }

        vs.push((bounds, v));

        let d = self.dimension;
        let avg_length =
          vs.iter().fold(
            0.0,
            |x, &(bounds, _)| x + length(&bounds, d)
          ) / NumCast::from(vs.len()).unwrap();

        if avg_length < length(&self.bounds, self.dimension) / 2.0 {
          self.loader.deref().borrow_mut().deref_mut().push(Unload(self.id));

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
      bounds: &AABB,
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

  fn on_ancestor<T>(&self, bounds: &AABB, f: |&Octree<V>| -> T) -> T {
    if self.bounds.contains(bounds) {
      f(self)
    } else {
      unsafe {
        assert!(self.parent.is_not_null());
        (*self.parent).on_ancestor(bounds, f)
      }
    }
  }

  fn on_mut_ancestor<T>(&mut self, bounds: &AABB, f: |&mut Octree<V>| -> T) -> T {
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
  pub fn intersect(&self, bounds: &AABB, self_v: V) -> bool {
    match self.contents {
      Leaf(ref vs) => vs.iter().any(|&(bs, ref v)| *v != self_v && bounds.intersects(&bs)),
      Branch(ref b) => {
        let mid = middle(&self.bounds, self.dimension);
        let (low_bounds, high_bounds) = split(mid, self.dimension, *bounds);
        low_bounds.map_or(false, |bs| b.low_tree.intersect(&bs, self_v)) ||
        high_bounds.map_or(false, |bs| b.high_tree.intersect(&bs, self_v))
      }
    }
  }

  // Find details of objects overlapping the object & bounds provided in this
  // and child trees. Uses equality comparison on V to ignore "same" objects.
  // Only finds intersects in this and child trees.
  pub fn intersect_details(&self, bounds: &AABB, self_v: V) -> HashSet<V> {
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
  pub fn intersect_details_from(&self, bounds: &AABB, self_v: V) -> HashSet<V> {
    self.on_ancestor(bounds, |t| t.intersect_details(bounds, self_v))
  }

  // like insert, but before recursing downward, we recurse up the parents
  // until the bounds provided are inside the tree.
  fn insert_from(&mut self, bounds: AABB, v: V) {
    self.on_mut_ancestor(&bounds, |t| t.insert(bounds.clone(), v))
  }

  pub fn remove(&mut self, v: V, bounds: &AABB) {
    assert!(self.bounds.contains(bounds));
    let collapse_contents = match self.contents {
      Leaf(ref mut vs) => {
        let i = vs.iter().position(|&(_, ref x)| *x == v).unwrap();
        vs.swap_remove(i);
        if vs.is_empty() {
          self.loader.deref().borrow_mut().deref_mut().push(Unload(self.id));
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

  pub fn reinsert(&mut self, v: V, bounds: &AABB, new_bounds: AABB) {
    self.remove(v, bounds);
    self.insert_from(new_bounds, v)
  }

  pub fn cast_ray(&self, ray: &Ray, self_v: V) -> Vec<V> {
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
