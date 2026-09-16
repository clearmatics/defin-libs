//! Utilities for working with different kind of caches.

pub mod linear_collections;
pub mod priority_set;
pub mod tls;

/// A macro to create a `NonZeroUsize` from a value, panicking if the value is zero.
/// For literal values, validation occurs at compile time. For expressions, validation
/// occurs at runtime.
#[macro_export]
macro_rules! NZUsize {
    ($val:literal) => {
        const { ::core::num::NonZeroUsize::new($val).expect("value must be non-zero") }
    };
    ($val:expr) => {
        // This will panic at runtime if $val is zero.
        ::core::num::NonZeroUsize::new($val).expect("value must be non-zero")
    };
}

/// A macro to create a `NonZeroU8` from a value, panicking if the value is zero.
/// For literal values, validation occurs at compile time. For expressions, validation
/// occurs at runtime.
#[macro_export]
macro_rules! NZU8 {
    ($val:literal) => {
        const { ::core::num::NonZeroU8::new($val).expect("value must be non-zero") }
    };
    ($val:expr) => {
        // This will panic at runtime if $val is zero.
        ::core::num::NonZeroU8::new($val).expect("value must be non-zero")
    };
}

/// A macro to create a `NonZeroU16` from a value, panicking if the value is zero.
/// For literal values, validation occurs at compile time. For expressions, validation
/// occurs at runtime.
#[macro_export]
macro_rules! NZU16 {
    ($val:literal) => {
        const { ::core::num::NonZeroU16::new($val).expect("value must be non-zero") }
    };
    ($val:expr) => {
        // This will panic at runtime if $val is zero.
        ::core::num::NonZeroU16::new($val).expect("value must be non-zero")
    };
}

/// A macro to create a `NonZeroU32` from a value, panicking if the value is zero.
/// For literal values, validation occurs at compile time. For expressions, validation
/// occurs at runtime.
#[macro_export]
macro_rules! NZU32 {
    ($val:literal) => {
        const { ::core::num::NonZeroU32::new($val).expect("value must be non-zero") }
    };
    ($val:expr) => {
        // This will panic at runtime if $val is zero.
        ::core::num::NonZeroU32::new($val).expect("value must be non-zero")
    };
}

/// A macro to create a `NonZeroU64` from a value, panicking if the value is zero.
/// For literal values, validation occurs at compile time. For expressions, validation
/// occurs at runtime.
#[macro_export]
macro_rules! NZU64 {
    ($val:literal) => {
        const { ::core::num::NonZeroU64::new($val).expect("value must be non-zero") }
    };
    ($val:expr) => {
        // This will panic at runtime if $val is zero.
        ::core::num::NonZeroU64::new($val).expect("value must be non-zero")
    };
}