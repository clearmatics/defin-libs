//! TLS (Thread Local Storage) utilities.
//!
//! These helpers wrapp the std TLS with an extra RAII guard for easy TLS returning.
//! When it is expensive to allocate memory for your thread localized data, it is recommended
//! to use these helper to allocate once without repeated allocation.
//!
//! [`TLSTaker`] is a RAII guard which its [`Drop`] function automatically returns the memory
//! to the TLS, so an explicit returning is not required anymore.
//!
//! # Concurrency
//!
//! These helpers are !Send, thus they are only for single thread context use case.
//!
//! # Examples
//!
//! ```
//! use utilities::cache::tls::{TLSTaker};
//! use utilities::thread_local_cache;
//! thread_local_cache!(static CACHE: String);
//! let cache = TLSTaker::take(&CACHE, || Ok::<_, ()>(String::new()), |s| {s.clear(); Ok(())} ).unwrap();
//! assert_eq!(&*cache, "");
//! drop(cache);
//!
//! // reuse the cached instance
//! let cache = TLSTaker::take(&CACHE, || Ok::<_, ()>(String::new()), |s| {s.clear(); Ok(())}).unwrap();
//! drop(cache);
//!
//! ```

use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::thread::LocalKey;

/// A TLSTaker with RAII guard, it takes the memry from the TLS slot and returns it on drop.
/// It is not Send, so it can be used only within a single thread context.
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
    /// `slot` pointer to the TLS.
    /// `new` the closure to allocate memory if the TLS isn't allocated.
    /// `reset` the closure to reset the memory if the TLS already exists.
    ///
    /// Taking a guard from the same TLS which was already held on the same thread will panic.
    pub fn take<E>(
        slot: &'static LocalKey<RefCell<(bool, Option<T>)>>,
        new: impl FnOnce() -> Result<T, E>,
        reset: impl FnOnce(&mut T) -> Result<(), E>,
    ) -> Result<Self, E> {
        // take the memory from the slot.
        let moved = slot.with(|cell| {
            let mut slot = cell.borrow_mut();
            assert!(!slot.0, "cache already held on this thread");
            slot.0 = true;
            slot.1.take()
        });

        let mut restorer = SlotRestorer { slot, taken: moved, hold: true };
        // anything failed in between below block will return the moved to the TLS.
        let value = match restorer.taken.take() {
            Some(mut v) => {
                if let Err(err) = reset(&mut v) {
                    // put the moved memory back.
                    restorer.taken = Some(v);
                    return Err(err);
                }
                v
            }
            None => new()?,
        };

        restorer.unhold();
        // after this, the returning is done by the dropper of TLSTaker
        // as it now take the moved value.
        Ok(Self { taken: Some(value), slot, _no_send: PhantomData })
    }
}

/// Drop returns the taken memory to the TLS.
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

/// SlotRestorer restores TLS state when the TLSTaker fails.
///
/// When holding data, its dropper clears the "hold" flag and
/// move the data back into the TLS slot.
struct SlotRestorer<T: 'static> {
    /// slot point to the TLS slot.
    slot: &'static LocalKey<RefCell<(bool, Option<T>)>>,
    /// the memory taken from the TLS.
    taken: Option<T>,
    /// the flag indicating memory taken or not.
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
            slot.1 = self.taken.take();
        });
    }
}

/// Create a thread-local slot for use with [`TLSTaker`].
///
/// ```ignore
/// thread_local_cache!(static SLOT: Type);
/// ```
///
/// Expands to a `thread_local!` with underlying layout:
/// `RefCell<(bool, Option<Type>)>` where:
/// - `(false, None)` means uninitialized
/// - `(false, Some(_))` means available
/// - `(true, None)` means held
#[macro_export]
macro_rules! thread_local_cache {
    (static $name:ident : $ty:ty) => {
        ::std::thread_local! {
            static $name: ::std::cell::RefCell<(bool, ::core::option::Option<$ty>)> =
                const { ::std::cell::RefCell::new((false, ::core::option::Option::None)) };
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    thread_local_cache!(static TEST_CACHE: Vec<u8>);

    #[test]
    fn test_new_allocation() {
        let cache =
            TLSTaker::take(&TEST_CACHE, || Ok::<_, ()>(vec![1, 2, 3, 4, 5]), |_v| Ok(())).unwrap();
        assert_eq!(&*cache, &[1, 2, 3, 4, 5]);
    }

    thread_local_cache!(static REUSE_CACHE: Vec<u8>);

    #[test]
    fn test_reuses_memory() {
        // allocation
        let mut cache = TLSTaker::take(
            &REUSE_CACHE,
            || Ok::<_, ()>(vec![1, 2, 3, 4, 5]),
            |v| {
                v.clear();
                Ok(())
            },
        )
        .unwrap();
        cache.push(6);
        drop(cache);

        // reuse, so the reset closure is call.
        let cache = TLSTaker::take(
            &REUSE_CACHE,
            || Ok::<_, ()>(vec![6]),
            |v| {
                v.clear();
                Ok(())
            },
        )
        .unwrap();
        assert!(cache.is_empty(), "reset should clear the vector");
    }

    thread_local_cache!(static RESET_ERR_CACHE: u32);

    #[test]
    fn test_reset_error() {
        // allocation
        {
            let _guard =
                TLSTaker::take(&RESET_ERR_CACHE, || Ok::<_, &str>(42), |_| Ok(())).unwrap();
        }

        // reuse,the reset should fail
        let result = TLSTaker::take(&RESET_ERR_CACHE, || Ok::<_, &str>(0), |_| Err("reset failed"));
        assert!(result.is_err());

        // the cached value should restore.
        let cached = RESET_ERR_CACHE.with(|cell| cell.borrow().1);
        assert_eq!(cached, Some(42));
    }

    thread_local_cache!(static ERR_CACHE: u32);

    #[test]
    fn test_new_error() {
        let result = TLSTaker::take(&ERR_CACHE, || Err::<u32, &str>("create failed"), |_| Ok(()));
        assert!(result.is_err());

        // A failed new should not leave the cache marked as held.
        let guard = TLSTaker::take(&ERR_CACHE, || Ok::<u32, &str>(7), |_| Ok(())).unwrap();
        assert_eq!(*guard, 7);
    }

    thread_local_cache!(static DROP_CACHE: String);

    #[test]
    fn test_drop_returns_to_tls() {
        {
            let _cache =
                TLSTaker::take(&DROP_CACHE, || Ok::<_, ()>(String::from("Hi!")), |_| Ok(()))
                    .unwrap();
            // _cache drops here
        }

        // Cache should now hold the value
        let has_value = DROP_CACHE.with(|cell| cell.borrow().1.is_some());
        assert!(has_value, "drop should return value to tls");
    }

    thread_local_cache!(static HELD_CACHE: Vec<u8>);

    #[test]
    fn test_panic_on_held_cache() {
        HELD_CACHE.with(|cell| *cell.borrow_mut() = (false, None));

        let result = std::panic::catch_unwind(|| {
            let mut outer =
                TLSTaker::take(&HELD_CACHE, || Ok::<_, ()>(vec![1]), |_| Ok(())).unwrap();
            outer.push(2);
            let _inner = TLSTaker::take(&HELD_CACHE, || Ok::<_, ()>(vec![3]), |_| Ok(())).unwrap();
        });
        assert!(result.is_err(), "nested take on same thread should panic");

        // Outer guard should have returned its value while unwinding.
        let cache = HELD_CACHE.with(|cell| cell.borrow().1.clone());
        assert_eq!(cache, Some(vec![1, 2]));
    }

    thread_local_cache!(static PANIC_CREATE_CACHE: u32);

    #[test]
    fn test_new_panic_no_impact_held() {
        let result = std::panic::catch_unwind(|| {
            let _ = TLSTaker::take(
                &PANIC_CREATE_CACHE,
                || -> Result<u32, ()> { panic!("new panic") },
                |_| Ok(()),
            );
        });
        assert!(result.is_err());

        let cache = TLSTaker::take(&PANIC_CREATE_CACHE, || Ok::<_, ()>(1), |_| Ok(())).unwrap();
        assert_eq!(*cache, 1);
    }

    thread_local_cache!(static PANIC_RESET_CACHE: u32);

    #[test]
    fn test_reset_panic_no_impact_held() {
        {
            let _cache =
                TLSTaker::take(&PANIC_RESET_CACHE, || Ok::<_, ()>(42), |_| Ok(())).unwrap();
        }

        let result = std::panic::catch_unwind(|| {
            let _ = TLSTaker::take(
                &PANIC_RESET_CACHE,
                || Ok::<_, ()>(0),
                |_| -> Result<(), ()> { panic!("reset panic") },
            );
        });
        assert!(result.is_err());

        let cache = TLSTaker::take(&PANIC_RESET_CACHE, || Ok::<_, ()>(9), |_| Ok(())).unwrap();
        assert_eq!(*cache, 9);
    }
}
