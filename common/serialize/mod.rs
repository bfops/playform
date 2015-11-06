//! Copy-based serialization functions. We don't use rustc-serialize
//! because it doesn't support bulk copies of Copy things.

// TODO: Make this work with less common endiannesses too.

#![deny(warnings)]
#![feature(convert)]
#![feature(raw)]
#![feature(vec_push_all)]

#![cfg_attr(test, feature(test))] 

extern crate num;

pub mod main;

pub use self::main::*;
