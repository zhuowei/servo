/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use heapsize::HeapSizeOf;
use refcell::{RefCell, Ref, RefMut};
use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

pub struct SingleThreadToken {
    not_send_or_sync: PhantomData<Rc<()>>,
}

impl SingleThreadToken {
    /// Returns a token that is safe to use if:
    ///
    /// * No other thread is using a `SingleThreadToken` at the same time
    /// * No `ReadOnlyToken` is used on any thread
    ///
    /// … with the same `StyleRefCell`s
    #[allow(unsafe_code)]
    pub unsafe fn assert() -> Self {
        SingleThreadToken {
            not_send_or_sync: PhantomData,
        }
    }
}

pub struct ReadOnlyToken(());

#[allow(unsafe_code)] unsafe impl Send for ReadOnlyToken {}
#[allow(unsafe_code)] unsafe impl Sync for ReadOnlyToken {}

impl ReadOnlyToken {
    /// Returns a token that is safe to use if `SingleThreadToken` is not used
    /// on any thread at the same time with the same `StyleRefCell`s.
    ///
    /// It is however safe for multiple threads to use `ReadOnlyToken` at the same time.
    #[allow(unsafe_code)]
    pub unsafe fn assert() -> ReadOnlyToken {
        ReadOnlyToken(())
    }
}

pub struct StyleRefCell<T> {
    refcell: RefCell<T>,
}

#[allow(unsafe_code)] unsafe impl<T: Send> Send for StyleRefCell<T> {}
#[allow(unsafe_code)] unsafe impl<T: Sync> Sync for StyleRefCell<T> {}

impl<T> StyleRefCell<T> {
    pub fn new(value: T) -> Self {
        StyleRefCell {
            refcell: RefCell::new(value),
        }
    }

    pub fn borrow_mut(&self, _token: &SingleThreadToken) -> RefMut<T> {
        self.refcell.borrow_mut()
    }

    pub fn borrow(&self, _token: &SingleThreadToken) -> Ref<T> {
        self.refcell.borrow()
    }

    #[allow(unsafe_code)]
    pub fn borrow_read_only(&self, _token: &ReadOnlyToken) -> &T {
        unsafe {
            &*self.refcell.as_ptr()
        }
    }
}

thread_local! {
    static UNIT_TESTING: Cell<bool> = Cell::new(false);
    static HEAP_SIZING: Cell<bool> = Cell::new(false);
}

/// Setting this flag allows `PartialEq for StyleRefCell` to be used.
/// (It is only used for unit testing.)
///
/// Doing so is safe when using `SingleThreadToken` is.
#[allow(unsafe_code)]
pub unsafe fn set_unit_testing_thread_local_flag(value: bool) {
    UNIT_TESTING.with(|flag| flag.set(value));
}

/// Setting this flag allows `HeapSizeOf for StyleRefCell` to be used.
///
/// Doing so is safe when using `SingleThreadToken` is.
#[allow(unsafe_code)]
pub unsafe fn set_heap_sizing_thread_local_flag(value: bool) {
    HEAP_SIZING.with(|flag| flag.set(value));
}

impl<T: PartialEq> PartialEq for StyleRefCell<T> {
    fn eq(&self, other: &Self) -> bool {
        UNIT_TESTING.with(|flag| assert!(flag.get()));
        *self.refcell.borrow() == *other.refcell.borrow()
    }
}

impl<T: HeapSizeOf> HeapSizeOf for StyleRefCell<T> {
    fn heap_size_of_children(&self) -> usize {
        // XXX: alternatively, remove the flag and always return zero
        HEAP_SIZING.with(|flag| assert!(flag.get()));
        self.refcell.borrow().heap_size_of_children()
    }
}

impl<T> fmt::Debug for StyleRefCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt("StyleRefCell", f)
    }
}
