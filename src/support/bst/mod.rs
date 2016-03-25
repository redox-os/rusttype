// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
#![allow(unused)]

use std::marker::PhantomData;
use std::ops::Deref;
use std::mem;
use std::fmt;

mod node;
mod search;
mod heap;
pub mod map;
pub mod set;

/// A wrapper around a raw `*mut T` that indicates that the possessor
/// of this wrapper owns the referent. This in turn implies that the
/// `Unique<T>` is `Send`/`Sync` if `T` is `Send`/`Sync`, unlike a raw
/// `*mut T` (which conveys no particular ownership semantics).  It
/// also implies that the referent of the pointer should not be
/// modified without a unique path to the `Unique` reference. Useful
/// for building abstractions like `Vec<T>` or `Box<T>`, which
/// internally use raw pointers to manage the memory that they own.
pub struct Unique<T: ?Sized> {
    pointer: NonZero<*const T>,
    // NOTE: this marker has no consequences for variance, but is necessary
    // for dropck to understand that we logically own a `T`.
    //
    // For details, see:
    // https://github.com/rust-lang/rfcs/blob/master/text/0769-sound-generic-drop.md#phantom-data
    _marker: PhantomData<T>,
}

/// `Unique` pointers are `Send` if `T` is `Send` because the data they
/// reference is unaliased. Note that this aliasing invariant is
/// unenforced by the type system; the abstraction using the
/// `Unique` must enforce it.
unsafe impl<T: Send + ?Sized> Send for Unique<T> {}

/// `Unique` pointers are `Sync` if `T` is `Sync` because the data they
/// reference is unaliased. Note that this aliasing invariant is
/// unenforced by the type system; the abstraction using the
/// `Unique` must enforce it.
unsafe impl<T: Sync + ?Sized> Sync for Unique<T> {}

impl<T: ?Sized> Unique<T> {
    /// Creates a new `Unique`.
    pub unsafe fn new(ptr: *mut T) -> Unique<T> {
        Unique {
            pointer: NonZero::new(ptr),
            _marker: PhantomData,
        }
    }

    /// Dereferences the content.
    pub unsafe fn get(&self) -> &T {
        &**self.pointer
    }

    /// Mutably dereferences the content.
    pub unsafe fn get_mut(&mut self) -> &mut T {
        &mut ***self
    }
}

impl<T: ?Sized> Deref for Unique<T> {
    type Target = *mut T;

    #[inline]
    fn deref(&self) -> &*mut T {
        unsafe { mem::transmute(&*self.pointer) }
    }
}

impl<T> fmt::Pointer for Unique<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&*self.pointer, f)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Bound<T> {
    /// An inclusive bound.
    Included(T),
    /// An exclusive bound.
    Excluded(T),
    /// An infinite endpoint. Indicates that there is no bound in this direction.
    Unbounded,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct NonZero<T>(T);

impl<T> NonZero<T> {
    unsafe fn new(inner: T) -> NonZero<T> {
        NonZero(inner)
    }
}

impl<T> ::std::ops::Deref for NonZero<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        let NonZero(ref inner) = *self;
        inner
    }
}

#[doc(hidden)]
trait Recover<Q: ?Sized> {
    type Key;

    fn get(&self, key: &Q) -> Option<&Self::Key>;
    fn take(&mut self, key: &Q) -> Option<Self::Key>;
    fn replace(&mut self, key: Self::Key) -> Option<Self::Key>;
}
