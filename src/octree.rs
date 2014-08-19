use common::*;
use nalgebra::na::{Vec3};
use ncollide3df32::bounding_volume::aabb::AABB;
use ncollide3df32::bounding_volume::BoundingVolume;
use ncollide3df32::ray::{Ray, RayCast};
use std::cell::RefCell;
use std::collections::{HashMap,HashSet};
use std::hash::Hash;
use std::mem;
use std::ptr::RawPtr;
use std::rc::Rc;
use vertex;
use vertex::ColoredVertex;
use glw::{GLBuffer,GLContext,Shader,Lines};

type F = f32;

fn length(bounds: &AABB, d: Dimension) -> F {
  get(d, bounds.maxs()) - get(d, bounds.mins())
}

fn middle(bounds: &AABB, d: Dimension) -> F {
  (get(d, bounds.maxs()) + get(d, bounds.mins())) / 2.0
}

fn get(d: Dimension, p: &Vec3<F>) -> F {
  match d {
    X => p.x,
    Y => p.y,
    Z => p.z,
  }
}

fn set<F>(d: Dimension, p: &mut Vec3<F>, v: F) {
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
    (Some(AABB::new(*bounds.mins(), new_max)),
      Some(AABB::new(new_min, *bounds.maxs())))
  }
}

pub enum Dimension { X, Y, Z }

#[deriving(Copy, PartialEq, Eq, Hash, Show)]
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

  outlines: GLBuffer<ColoredVertex>,
}

impl<V> OctreeBuffers<V> {
  pub unsafe fn new(gl: &GLContext, shader_program: &Rc<Shader>) -> OctreeBuffers<V> {
    OctreeBuffers {
      entry_to_index: HashMap::new(),
      index_to_entry: Vec::new(),

      outlines: GLBuffer::new(
        gl,
        shader_program.clone(),
        [ vertex::AttribData { name: "position", size: 3 },
          vertex::AttribData { name: "in_color", size: 4 },
        ],
        LINE_VERTICES_PER_BOX,
        1 << 20,
        Lines
      ),
    }
  }

  pub fn push(
    &mut self,
    entry: OctreeId,
    outlines: &[ColoredVertex]
  ) {
    self.entry_to_index.insert(entry, self.index_to_entry.len());
    self.index_to_entry.push(entry);

    self.outlines.push(outlines);
  }

  pub fn flush(&mut self, gl: &GLContext) {
    self.outlines.flush(gl);
  }

  pub fn swap_remove(&mut self, gl: &GLContext, entry: OctreeId) {
    match self.entry_to_index.find(&entry) {
      None => {},
      Some(&idx) => {
        let swapped_id = self.index_to_entry[self.index_to_entry.len() - 1];
        self.index_to_entry.swap_remove(idx).unwrap();
        self.entry_to_index.remove(&entry);
        self.outlines.swap_remove(gl, idx);
        if entry != swapped_id {
          self.entry_to_index.insert(swapped_id, idx);
        }
      },
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

type LeafContents<V> = Vec<(AABB, OctreeId, V)>;

// TODO: try real hard to remove the Empty case.
enum OctreeContents<V> {
  Empty,
  Leaf(LeafContents<V>),
  Branch(Branches<V>),
}

static mut next_id: OctreeId = OctreeId(0);

// TODO: allow inserting things with a "mobile" flag; don't subdivide those objects.
pub struct Octree<V> {
  parent: *mut Octree<V>,
  dimension: Dimension,
  bounds: AABB,
  contents: OctreeContents<V>,

  // for rendering
  id: OctreeId,
  buffers: Rc<RefCell<OctreeBuffers<V>>>,
}

// TODO: fix shaky octree outline insertion/removal conditions.

impl<V: Copy + Eq + PartialOrd + Hash> Octree<V> {
  pub fn new(buffers: &Rc<RefCell<OctreeBuffers<V>>>, bounds: &AABB) -> Octree<V> {
    Octree {
      parent: RawPtr::null(),
      dimension: X,
      bounds: bounds.clone(),
      contents: Empty,
      id: Octree::<V>::alloc_id(),
      buffers: buffers.clone(),
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

  pub fn insert(&mut self, gl: &GLContext, bounds: AABB, v: V) -> *mut Octree<V> {
    assert!(self.bounds.contains(&bounds));
    let t: Option<*mut Octree<V>> =
      match self.contents {
        Empty => {
          let id = Octree::<V>::alloc_id();
          let vs = Vec::from_fn(1, |_| (bounds.clone(), id, v));
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
                  low.insert(gl, bs, v);
                  spans_multiple = true;
                  parent = Some(mem::transmute(&mut *low));
                }
              );
              high_bounds.map(
                |bs| {
                  high.insert(gl, bs, v);
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
            self.buffers.deref().borrow_mut().deref_mut().push(self.id, to_outlines(&self.bounds));
            self.buffers.deref().borrow_mut().deref_mut().push(id, to_outlines(&bounds));
            self.buffers.deref().borrow_mut().deref_mut().flush(gl);
            None
          }
        },
        Leaf(ref mut vs) => {
          let id = Octree::<V>::alloc_id();
          vs.push((bounds, id, v));
          self.buffers.deref().borrow_mut().deref_mut().push(id, to_outlines(&bounds));
            self.buffers.deref().borrow_mut().deref_mut().flush(gl);
          // TODO: bisect this leaf if the median length is less than half the
          // current length.
          None
        },
        Branch(ref mut b) => {
          // copied in remove()
          let (l, h) = split(middle(&self.bounds, self.dimension), self.dimension, bounds);
          l.map(|low_half| b.low_tree.insert(gl, low_half, v));
          h.map(|high_half| b.high_tree.insert(gl, high_half, v));
          None
        },
      };
    unsafe {
      t.map_or(mem::transmute(self), |x| mem::transmute(x))
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
        id: Octree::<V>::alloc_id(),
        buffers: self.buffers.clone(),
      },
      Octree {
        parent: self,
        dimension: new_d,
        bounds: high_bounds.clone(),
        contents: Empty,
        id: Octree::<V>::alloc_id(),
        buffers: self.buffers.clone(),
      }
    )
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
      Empty => false,
      Leaf(ref vs) => vs.iter().any(|&(bs, _, ref v)| *v != self_v && bounds.intersects(&bs)),
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
      Empty => HashSet::new(),
      Leaf(ref vs) => {
        let mut r = HashSet::new();
        for &(bs, _, v) in vs.iter() {
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
  fn insert_from(&mut self, gl: &GLContext, bounds: AABB, v: V) -> *mut Octree<V> {
    self.on_mut_ancestor(&bounds, |t| t.insert(gl, bounds.clone(), v))
  }

  pub fn remove(&mut self, gl: &GLContext, v: V, bounds: &AABB) {
    assert!(self.bounds.contains(bounds));
    let collapse_contents = match self.contents {
      Empty => {
        fail!("Could not Octree::remove(&Empty)");
      },
      Leaf(ref mut vs) => {
        let i = vs.iter().position(|&(_, _, ref x)| *x == v).expect("could not Octree::remove()");
        let (_, id, _) = (*vs)[i];
        self.buffers.deref().borrow_mut().deref_mut().swap_remove(gl, id);
        vs.swap_remove(i);
        if vs.is_empty() {
          self.buffers.deref().borrow_mut().deref_mut().swap_remove(gl, self.id);
          true
        } else {
          false
        }
      },
      Branch(ref mut bs) => {
        let (l, h) = split(middle(&self.bounds, self.dimension), self.dimension, *bounds);
        l.map(|low_half| bs.low_tree.remove(gl, v, &low_half));
        h.map(|high_half| bs.high_tree.remove(gl, v, &high_half));
        bs.low_tree.is_empty() && bs.high_tree.is_empty()
      }
    };

    if collapse_contents {
      self.contents = Empty;
    }
  }

  pub fn is_empty(&self) -> bool {
    match self.contents {
      Empty => true,
      _ => false,
    }
  }

  pub fn move(&mut self, gl: &GLContext, v: V, bounds: &AABB, new_bounds: AABB) -> *mut Octree<V> {
    self.remove(gl, v, bounds);
    self.insert_from(gl, new_bounds, v)
  }

  pub fn cast_ray(&self, ray: &Ray, self_v: V) -> Option<V> {
    match self.contents {
      Empty => {
        None
      },
      Leaf(ref vs) => {
        // find the time of intersection (TOI) of the ray with each object in
        // this leaf; filter out the objects it doesn't intersect at all. Then
        // find the object with the lowest TOI.
        partial_min_by(
          vs.iter().filter_map(|&(bounds, _, v)| {
              if v == self_v {
                None
              } else {
                bounds.toi_with_ray(ray, true).map(|x| (x, v))
              }
            }
          ),
          |(toi, _)| toi
        )
        .map(|(_, v)| v)
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
          if r.is_some() {
            return r;
          }
        }
        None
      }
    }
  }
}
