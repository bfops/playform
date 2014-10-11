extern crate rlibc;

use std::default::Default;
use std::iter::Chain;
use std::mem;
use std::slice;

fn vec_copy<T>(v: &mut Vec<T>, to: uint, from: uint, n: uint) {
  unsafe {
    let p_to = v.as_mut_ptr().offset(to as int);
    let p_from = v.as_ptr().offset(from as int);
    rlibc::memcpy(p_to as *mut u8, p_from as *const u8, n * mem::size_of::<T>());
  }
}

/// Circular bounded queue.
pub struct Queue<T> {
  pub contents: Vec<T>,
  /// index in `contents` the queue starts at
  pub head: uint,
  /// index in `contents` to place the next element at
  pub tail: uint,
  /// number of elements used
  pub length: uint,
}

impl<T: Clone> Queue<T> {
  pub fn new(capacity: uint) -> Queue<T> {
    Queue {
      contents: Vec::with_capacity(capacity),
      head: 0,
      tail: 0,
      length: 0,
    }
  }

  pub fn push(&mut self, t: T) {
    assert!(
      self.length < self.contents.capacity(),
      "Bounded queue (capacity {}) exceeded",
      self.contents.capacity()
    );

    if self.tail >= self.contents.len() {
      assert!(self.contents.len() < self.contents.capacity());
      self.contents.push(t);
    } else {
      *self.contents.get_mut(self.tail) = t;
    }

    self.tail = (self.tail + 1) % self.contents.capacity();
    self.length += 1;
  }

  pub fn push_all<'a, I: Iterator<T>>(&mut self, ts: I) {
    let mut ts = ts;
    for t in ts {
      self.push(t);
    }
  }

  pub fn pop(&mut self, count: uint) {
    assert!(count <= self.length);
    self.head = (self.head + count) % self.contents.capacity();
    self.length -= count;
  }

  pub fn is_empty(&self) -> bool {
    self.length == 0
  }

  /// Swap `count` elements from `idx` with the last `count` elements of the
  /// queue, then drop the last `count` elements.
  /// The ordering amongst those `count` elements is maintained.
  pub fn swap_remove(&mut self, idx: uint, count: uint) {
    assert!(count <= self.length);
    self.length -= count;

    assert!(idx <= self.length);

    self.tail = (self.tail + self.contents.capacity() - count) % self.contents.capacity();

    if idx < self.length {
      assert!(
        idx + count <= self.length,
        "Queue::swap_remove in overlapping regions"
      );

      let idx = idx + self.head;

      // At this point, we have a guarantee that the regions do not overlap.
      // Therefore, either the copy-from or copy-to regions might be broken up
      // by the end of the vector, but not both.

      let buffer_wrap_point = self.contents.capacity() - count;
      if idx > buffer_wrap_point {
        assert!(self.contents.len() == self.contents.capacity());

        // copy TO an area of the queue that crosses the end of the buffer.

        let count1 = self.contents.capacity() - idx;
        vec_copy(&mut self.contents, idx, self.tail, count1);
        let count2 = count - count1;
        vec_copy(&mut self.contents, 0, self.tail + count1, count2);
      } else if self.tail > buffer_wrap_point {
        assert!(self.contents.len() == self.contents.capacity());

        // copy FROM an area of the queue that crosses the end of the buffer.

        let count1 = self.contents.capacity() - self.tail;
        vec_copy(&mut self.contents, idx, self.tail, count1);
        let count2 = count - count1;
        vec_copy(&mut self.contents, idx + count1, 0, count2);
      } else {
        vec_copy(&mut self.contents, idx, self.tail, count);
      }
    }
  }

  pub fn len(&self) -> uint {
    self.length
  }

  #[allow(dead_code)]
  pub fn clear(&mut self) {
    self.contents.clear();
    self.head = 0;
    self.tail = 0;
    self.length = 0;
  }

  pub fn iter<'a>(&'a self, low: uint, high: uint) -> QueueItems<'a, T> {
    let (l, h) = self.slices(low, high);
    QueueItems { inner: l.iter().chain(h.iter()) }
  }

  pub fn slices<'a>(&'a self, low: uint, high: uint) -> (&'a [T], &'a [T]) {
    assert!(low <= high);
    assert!(low <= self.length);
    assert!(high <= self.length);
    let (l, h) =
      if low == high {
        (Default::default(), Default::default())
      } else {
        let head = (self.head + low) % self.contents.capacity();
        let tail = (self.head + high) % self.contents.capacity();
        if head < tail {
          (self.contents.slice(head, tail), Default::default())
        } else {
          assert!(self.contents.len() == self.contents.capacity());
          (self.contents.slice(head, self.contents.capacity()), self.contents.slice(0, tail))
        }
      };
    assert!(l.len() + h.len() == high - low);
    (l, h)
  }
}

#[test]
fn push_then_slice() {
  let elems: Vec<int> = Vec::from_slice([1, 2, 3]);
  let mut q = Queue::new(32);
  q.push_all(elems.iter());
  assert!(q.len() == elems.len());
  let (l, h) = q.slices(0, q.len() - 1);
  assert!(l.len() + h.len() == elems.len() - 1);
  for (i, elem) in l.iter().enumerate() {
    assert!(**elem == elems[i]);
  }
  for (i, elem) in h.iter().enumerate() {
    assert!(**elem == elems[i + l.len()]);
  }
}

#[test]
fn push_then_pop() {
  let popped_pushes: Vec<int> = Vec::from_slice([1, 2, 3]);
  let more_pushes: Vec<int> = Vec::from_slice([4, 5]);
  let mut q = Queue::new(32);
  q.push_all(popped_pushes.iter());
  q.push_all(more_pushes.iter());
  assert!(q.len() == popped_pushes.len() + more_pushes.len());
  q.pop(popped_pushes.len());
  assert!(q.len() == more_pushes.len());
  let (l, h) = q.slices(0, q.len());
  assert!(l.len() + h.len() == more_pushes.len());
  for (i, elem) in l.iter().enumerate() {
    assert!(**elem == more_pushes[i]);
  }
  for (i, elem) in h.iter().enumerate() {
    assert!(**elem == more_pushes[i + l.len()]);
  }
}

#[test]
fn wrapped_pushes() {
  static capacity: uint = 32;
  let popped_pushes = Vec::from_elem(capacity / 2, 0i);
  let more_pushes: Vec<int> = range_inclusive(1, capacity as int).collect();
  let mut q: Queue<int> = Queue::new(capacity);
  q.push_all(popped_pushes.iter().map(|&x| x));
  q.pop(popped_pushes.len());
  q.push_all(more_pushes.iter().map(|&x| x));
  assert!(q.len() == more_pushes.len());
  let (l, h) = q.slices(0, q.len());
  assert!(l.len() + h.len() == more_pushes.len());
  assert!(h.len() > 0);
  for (x, y) in l.iter().chain(h.iter()).zip(more_pushes.iter()) {
    assert!(x == y);
  }
}

#[test]
// TODO: clean up this test, de-duplicate, etc.
fn swap_remove_simple() {
  let mut q: Queue<int> = Queue::new(32);
  q.push_all([1,2,3,4,7,8,5,6i].iter().map(|&x| x));
  q.swap_remove(4, 2);

  let i = q.iter(0, q.len());
  let elems = [1,2,3,4,5,6i];
  let r = elems.iter();
  assert!(i.size_hint() == r.size_hint());
  for (x, y) in i.zip(r) {
    assert!(x == y);
  }
}

#[test]
// TODO: clean up this test, de-duplicate, etc.
fn push_swap_remove_push() {
  let mut q: Queue<int> = Queue::new(32);
  q.push_all([1, 9i].iter().map(|&x| x));
  q.swap_remove(1, 1);
  q.push_all([2, 3, 4, 5i].iter().map(|&x| x));

  let elems = [1, 2, 3, 4, 5i];
  let i = q.iter(0, q.len());
  let mut count = 0;
  for (i, &x) in i.enumerate() {
    assert!(i < elems.len());
    assert_eq!(x, elems[i]);
    count += 1;
  }
  assert!(count == q.len());
  assert!(count == elems.len());
}

#[test]
// TODO: clean up this test, de-duplicate, etc.
fn swap_remove_end() {
  let mut q: Queue<int> = Queue::new(8);
  let popped = [11, 12i];
  let elems = [1,2,3,4,5,6,7,8i];

  q.push_all(popped.iter().map(|&x| x));
  q.pop(popped.len());
  q.push_all(elems.iter().map(|&x| x));
  q.swap_remove(4, 4);
  assert!(q.len() == 4);

  let i = q.iter(0, q.len());
  let mut count = 0;
  for (i, &x) in i.enumerate() {
    assert!(i < elems.len());
    assert!(x == elems[i]);
    count += 1;
  }
  assert!(count == q.len());
}

// TODO: clean up this test, de-duplicate, etc.
#[test]
fn swap_remove_to_broken() {
  let mut q: Queue<int> = Queue::new(8);
  let popped = [9,9,9,9i];
  q.push_all(popped.iter().map(|&x| x));
  q.pop(popped.len());
  q.push_all([1,2,3,7,8,6,4,5i].iter().map(|&x| x));
  q.swap_remove(3, 2);

  let i = q.iter(0, q.len());
  let elems = [1,2,3,4,5,6i];
  let r = elems.iter();
  assert!(i.size_hint() == r.size_hint());
  for (x, y) in i.zip(r) {
    assert!(x == y);
  }
}

#[test]
// TODO: clean up this test, de-duplicate, etc.
fn swap_remove_from_broken() {
  let mut q: Queue<int> = Queue::new(8);
  let popped = [9,9,9,9i];
  q.push_all(popped.iter().map(|&x| x));
  q.pop(popped.len());
  q.push_all([4,5,6,1,2,3i].iter().map(|&x| x));
  q.swap_remove(0, 3);

  let i = q.iter(0, q.len());
  let elems = [1,2,3i];
  let r = elems.iter();
  assert!(i.size_hint() == r.size_hint());
  for (x, y) in i.zip(r) {
    assert!(x == y);
  }
}

#[test]
fn vec_copy_test() {
  let mut v = Vec::from_slice([1,2,3,4,5i]);
  vec_copy(&mut v, 1, 3, 2);
  assert_eq!(v, Vec::from_slice([1, 4, 5, 4, 5i]));
}

pub struct QueueItems<'a, T:'a> { inner: Chain<slice::Items<'a, T>, slice::Items<'a, T>> }

impl<'a, T> Iterator<&'a T> for QueueItems<'a, T> {
  fn next(&mut self) -> Option<&'a T> { self.inner.next() }
  fn size_hint(&self) -> (uint, Option<uint>) { self.inner.size_hint() }
}
