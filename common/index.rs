//! Strongly typed buffer index type

use std;

#[allow(missing_docs)]
#[derive(Debug)]
pub struct T<Buffer, Element> {
  phantom : std::marker::PhantomData<(Buffer, Element)>,
  index   : u32,
}

/// Assert that an index is a valid element index in a buffer.
pub fn of_u32<Buffer, Element>(index: u32) -> T<Buffer, Element> {
  T {
    phantom : std::marker::PhantomData,
    index   : index,
  }
}

impl<Buffer, Element> Clone for T<Buffer, Element> where Buffer: Sized, Element: Sized {
  fn clone(&self) -> Self {
    of_u32(self.index)
  }
}

impl<Buffer, Element> Copy for T<Buffer, Element> where Buffer: Sized, Element: Sized { }

fn length<Buffer, Element>() -> u32 {
  let this_size = std::mem::size_of::<Buffer>() as u32;
  let sub_size = std::mem::size_of::<Element>() as u32;
  assert_eq!(this_size % sub_size, 0, "Element does not cleanly divide Buffer");
  this_size / sub_size
}

#[allow(missing_docs)]
pub struct All<Buffer, Element> {
  phantom : std::marker::PhantomData<(Buffer, Element)>,
  indices : std::ops::Range<u32>,
}

/// Iterate through all the indices in a buffer
pub fn all<Buffer, Element>() -> All<Buffer, Element> {
  All {
    phantom : std::marker::PhantomData,
    indices : (0..length::<Buffer, Element>()),
  }
}

impl<Buffer, Element> Iterator for All<Buffer, Element> {
  type Item = T<Buffer, Element>;
  fn next(&mut self) -> Option<Self::Item> {
    self.indices.next().map(|i| of_u32(i))
  }
}

impl<Buffer, Element> T<Buffer, Element> {
  /// Cast to an index of more granular elements.
  pub fn downcast<SubElement>(self) -> T<Buffer, SubElement> {
    of_u32(self.index * length::<Element, SubElement>())
  }

  /// Index the Element type itself
  pub fn subindex<Subindex>(self, subindex: T<Element, Subindex>) -> T<Buffer, Subindex> {
    let downcast: T<Buffer, Subindex> = self.downcast();
    of_u32(downcast.index + subindex.index)
  }

  /// Strip type information from this index.
  pub fn to_u32(self) -> u32 {
    self.index
  }
}
