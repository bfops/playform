use num;
use std::mem;
use std::raw;

/// End-of-stream error.
#[derive(Debug)]
pub struct EOF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Copyable<T>(pub T) where T: Copy;

// TODO: This can probably be replaced with std things.
/// "Seek" over an in-memory byte slice.
pub struct MemStream<'a> {
  data: &'a [u8],
  position: usize,
}

impl<'a> MemStream<'a> {
  /// Initialize a MemStream to read from the beginning of the provided slice.
  pub fn new(data: &'a [u8]) -> MemStream<'a> {
    MemStream {
      data: data,
      position: 0,
    }
  }

  /// Read a series of bytes off the slice. This is mainly a slicing operation.
  pub fn take(&mut self, len: usize) -> Result<&'a [u8], EOF> {
    if self.data.len() < self.position + len {
      return Err(EOF)
    }

    let old_position = self.position;
    self.position += len;
    let r = &self.data[old_position .. self.position];
    Ok(r)
  }
}

/// Shortcut function for Flatten::emit with `Vec::new()`.
pub fn encode<T>(v: &T) -> Result<Vec<u8>, ()> where T: Flatten {
  let mut r = Vec::new();
  try!(Flatten::emit(v, &mut r));
  Ok(r)
}

/// Shortcut function for Flatten::read.
pub fn decode<T>(data: &[u8]) -> Result<T, EOF> where T: Flatten {
  let mut memstream = MemStream::new(data);
  Flatten::read(&mut memstream)
}

/// Emit a value's (shallow) raw bytes into the destination Vec.
pub fn emit_as_bytes<T>(v: &T, dest: &mut Vec<u8>) -> Result<(), ()> {
  let bytes = unsafe {
    mem::transmute(
      raw::Slice {
        data: v as *const T as *const u8,
        len: mem::size_of::<T>(),
      }
    )
  };

  dest.push_all(bytes);
  Ok(())
}

/// Parse a value by reading its (shallow) raw bytes directly.
pub fn of_bytes<'a, T>(bytes: &mut MemStream<'a>) -> Result<T, EOF> where T: Copy {
  let bytes = try!(bytes.take(mem::size_of::<T>()));

  let v = bytes.as_ptr() as *const T;
  let v = unsafe { *v };
  Ok(v)
}

/// [De]serialization functions based around flattening via Copy.
pub trait Flatten: Sized {
  /// Emit a value into a Vec of bytes.
  fn emit(v: &Self, dest: &mut Vec<u8>) -> Result<(), ()>;
  /// Parse a value out of a stream of bytes.
  fn read<'a>(s: &mut MemStream<'a>) -> Result<Self, EOF>;
}

fn emit_slice_as_bytes<T>(v: &[T], dest: &mut Vec<u8>) -> Result<(), ()> {
  let len: u32;
  match num::NumCast::from(v.len()) {
    None => return Err(()),
    Some(l) => len = l,
  }

  let len = Copyable(len);
  try!(Flatten::emit(&len, dest));

  let bytes = unsafe {
    mem::transmute(
      raw::Slice {
        data: v.as_ptr() as *const u8,
        len: v.len() * mem::size_of::<T>(),
      }
    )
  };

  dest.push_all(bytes);
  Ok(())
}

impl<T> Flatten for Vec<T> where T: Copy {
  fn emit(v: &Vec<T>, dest: &mut Vec<u8>) -> Result<(), ()> {
    emit_slice_as_bytes(v.as_slice(), dest)
  }

  fn read<'a>(s: &mut MemStream<'a>) -> Result<Vec<T>, EOF> {
    let len: Copyable<u32> = try!(Flatten::read(s));
    let len = len.0 as usize;

    let slice = try!(s.take(len * mem::size_of::<T>()));
    let slice: &[T] = unsafe {
      mem::transmute(
        raw::Slice {
          data: slice.as_ptr() as *const T,
          len: len,
        }
      )
    };

    Ok(slice.iter().map(|&x| x).collect())
  }
}

impl Flatten for String {
  fn emit(v: &String, dest: &mut Vec<u8>) -> Result<(), ()> {
    emit_slice_as_bytes(v.as_bytes(), dest)
  }

  fn read<'a>(s: &mut MemStream<'a>) -> Result<String, EOF> {
    let v = try!(Flatten::read(s));
    let s = unsafe {
      String::from_utf8_unchecked(v)
    };
    Ok(s)
  }
}

impl<T> Flatten for Copyable<T> where T: Copy {
  fn emit(v: &Copyable<T>, dest: &mut Vec<u8>) -> Result<(), ()> {
    emit_as_bytes(v, dest)
  }

  fn read<'a>(v: &mut MemStream<'a>) -> Result<Copyable<T>, EOF> {
    of_bytes(v).map(|v| Copyable(v))
  }
}

#[macro_export]
macro_rules! flatten_unit_struct_impl(
  ( $name: ident ) => {
    impl Flatten for $name {
      fn emit(v: &$name, dest: &mut Vec<u8>) -> Result<(), ()> {
        try!(Flatten::emit(&v.0, dest));
        Ok(())
      }
      fn read<'a>(s: &mut MemStream<'a>) -> Result<$name, EOF> {
        let member = try!(Flatten::read(s));
        Ok($name(member))
      }
    }
  }
);

#[macro_export]
macro_rules! flatten_struct_impl(
  ( $name: ident, $( $member: ident),* ) => {
    impl Flatten for $name {
      fn emit(v: &$name, dest: &mut Vec<u8>) -> Result<(), ()> {
        $( try!(Flatten::emit(&v.$member, dest)); )*
        Ok(())
      }
      fn read<'a>(s: &mut MemStream<'a>) -> Result<$name, EOF> {
        $( let $member = try!(Flatten::read(s)); )*
        Ok($name {
          $( $member: $member, )*
        })
      }
    }
  }
);

#[macro_export]
macro_rules! flatten_enum_impl(
  ( $name: ident, $tag: ty, $( ( $variant: ident, $in_id: pat, $out_id: expr, $( $field: ident ),* ), )* ) => {
    impl Flatten for $name {
      fn emit(v: &$name, dest: &mut Vec<u8>) -> Result<(), ()> {
        match *v {
          $(
            $name::$variant( $( ref $field, )* ) => {
              let tag: $tag = $out_id;
              try!(Flatten::emit(&tag, dest));
              $(
                try!(Flatten::emit($field, dest));
              )*
            },
          )*
        }
        Ok(())
      }

      fn read<'a>(s: &mut MemStream<'a>) -> Result<$name, EOF> {
        let tag: $tag = try!(Flatten::read(s));
        match tag {
          $(
            $in_id => {
              $(
                let $field = try!(Flatten::read(s));
              )*
              Ok($name::$variant( $( $field, )* ))
            },
          )*
          _ => panic!("Invalid tag"),
        }
      }
    }
  }
);

#[cfg(test)]
mod tests {
  extern crate test;

  use super::{Flatten, Copyable, MemStream, EOF, encode, decode};

  #[derive(Debug, PartialEq, Eq)]
  struct Foo {
    data: Vec<(i32, u64)>,
    t: Copyable<i8>,
  }

  flatten_struct_impl!(Foo, data, t);

  #[derive(Debug, PartialEq, Eq)]
  struct Bar {
    t: Copyable<u32>,
    items: Foo,
  }

  flatten_struct_impl!(Bar, t, items);

  #[derive(Debug, PartialEq, Eq)]
  struct Baz {
    foo: Foo,
    thing: Copyable<i8>,
    bar: Bar,
  }

  flatten_struct_impl!(Baz, foo, thing, bar);

  #[test]
  fn simple_test() {
    let baz =
      Baz {
        foo: Foo {
          data: vec!((1, 1), (2, 4), (3, 255)),
          t: Copyable(118),
        },
        thing: Copyable(3),
        bar: Bar {
          t: Copyable(7),
          items: Foo {
            data: vec!((0, 8), (3, 9), (6, 10)),
            t: Copyable(-3),
          },
        },
      };
    let encoded = encode(&baz).unwrap();
    println!("encoded baz: {:?}", encoded);
    let rebaz = decode(encoded.as_slice()).unwrap();
    assert_eq!(baz, rebaz);
  }

  #[bench]
  fn bench_encode_decode_binary(_: &mut test::Bencher) {
    let baz =
      Baz {
        foo: Foo {
          data: vec!((1, 1), (2, 4), (3, 255)),
          t: Copyable(118),
        },
        thing: Copyable(3),
        bar: Bar {
          t: Copyable(7),
          items: Foo {
            data: vec!((0, 8), (3, 9), (6, 10)),
            t: Copyable(-3),
          },
        },
      };
    let encoded = encode(&baz).unwrap();
    let rebaz: Baz = decode(encoded.as_slice()).unwrap();
    test::black_box(rebaz);
  }
}
