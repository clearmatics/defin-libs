//! TLS (Thread Local Storage) utilities.
//!
//! These helpers wrapp the std TLS with an extra RAII guard for easy TLS operation.
//! When it is expensive to allocate memory for your thread localized data, it is recommended
//! to use these helper to allocate once without repeated allocation.
//!
//! [`TLSTaker`] is an RAII guard which its [`Drop`] function automatically returns the memory
//! to the TLS, so an explicit returning is not required anymore.
//!
//! # Concurrency
//!
//! These helpers are not Send, thus they are only for single thread context.
//!
//! # Examples
//!

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::thread::LocalKey;

/// A TLSTaker works in RAII guard mode, it takes the memry from the TLS slot and returns it on drop.
/// It is not Send, thus that it can be used within single thread context.
pub struct TLSTaker<T: 'static> {
    /// the memory taken from the TLS.
    taken: Option<T>,
    /// the TLS pointer
    slot: &'static LocalKey<RefCell<(bool, Option<T>)>>,
    /// mark it as !Send
    _no_send: PhantomData<*const ()>,
}

impl<T: 'static> TLSTaker<T> {
    /// Take the memory from the TLS.
    ///
    /// `cache` pointer to the TLS.
    /// `new` the closure to allocate memory if the TLS isn't initialized.
    /// `reset` the closure to reset the memory if the TLS already exists.
    ///
    /// Taking a guard from the same TLS which was already held on the same thread will panic.
    pub fn take<E>(
        cache: &'static LocalKey<RefCell<(bool, Option<T>)>>,
        new: impl FnOnce() -> Result<T, E>,
        reset: impl FnOnce(&mut T) -> Result<(), E>,
    ) -> Result<Self, E> {
        // take the memory from the slot.
        let moved = cache.with(|cell| {
            let mut slot = cell.borrow_mut();
            assert!(!slot.0, "cache already held on this thread");
            slot.0 = true;
            slot.1.take()
        });

        let mut restorer = SlotRestorer { slot: cache, data: moved, hold: true };
        let value = match restorer.data.take() {
            Some(mut v) => {
                if let Err(err) = reset(&mut v) {
                    restorer.data = Some(v);
                    return Err(err);
                }
                v
            }
            None => new()?,
        };

        restorer.unhold();
        Ok(Self { taken: Some(value), slot: cache, _no_send: PhantomData })
    }
}

impl<T: 'static> Drop for TLSTaker<T> {
    fn drop(&mut self) {
        if let Some(v) = self.taken.take() {
            self.slot.with(|cell| {
                let mut slot = cell.borrow_mut();
                assert!(slot.0, "cache should be held");
                slot.0 = false;
                slot.1 = Some(v);
            })
        }
    }
}

impl<T: 'static> Deref for TLSTaker<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.taken.as_ref().expect("should not take after drop")
    }
}

impl<T: 'static> DerefMut for TLSTaker<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.taken.as_mut().expect("should not take after drop")
    }
}

/// SlotRestorer restores TLS state when the Getter fails.
///
/// When holding data, its dropper clears the "hold" flag and
/// move the data back into the TLS slot.
struct SlotRestorer<T: 'static> {
    /// slot point to the TLS slot.
    slot: &'static LocalKey<RefCell<(bool, Option<T>)>>,
    /// the data replica
    data: Option<T>,
    hold: bool,
}

impl<T: 'static> SlotRestorer<T> {
    const fn unhold(&mut self) {
        self.hold = false;
    }
}

impl<T: 'static> Drop for SlotRestorer<T> {
    fn drop(&mut self) {
        // do not restore if it does not hold data.
        if !self.hold {
            return;
        }

        self.slot.with(|cell| {
            let mut slot = cell.borrow_mut();
            assert!(slot.0, "cache should be held");
            slot.0 = false;
            slot.1 = self.data.take();
        });
    }
}

#[macro_export]
macro_rules! thread_local_cache {
    (static $name:ident : $ty:ty) => {
        ::std::thread_local! {
            static $name: ::std::cell::RefCell<(bool, ::core::option::Option<$ty>)> =
                const { ::std::cell::RefCell::new((false, ::core::option::Option::None)) };
        }
    };
}
