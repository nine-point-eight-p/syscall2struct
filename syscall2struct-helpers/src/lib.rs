#![no_std]

use enum_index_derive::EnumIndex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(not(feature = "alloc"))]
const DEFAULT_MAP_SIZE: usize = 32;

#[cfg(not(feature = "alloc"))]
type Map<K, V, const N: usize = DEFAULT_MAP_SIZE> = heapless::FnvIndexMap<K, V, N>;

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
type Map<K, V> = alloc::collections::BTreeMap<K, V>;

/// Make a syscall with self-contained arguments.
pub trait MakeSyscall {
    /// Syscall number
    const NR: i32;

    /// Make a syscall with arguments and previous syscall results.
    fn call(&self, results: &ResultContainer) -> isize;
}

/// Make a syscall with self-contained arguments.
/// The arguments may be modified to receive data from the syscall.
pub trait MakeSyscallMut {
    /// Syscall number
    const NR: i32;

    /// Make a syscall with arguments and previous syscall results.
    /// The arguments may be modified to receive data from the syscall.
    fn call(&mut self, results: &ResultContainer) -> isize;
}

/// Convert to a pointer.
pub trait AsPtr<T: ?Sized> {
    fn as_ptr(&self) -> *const T;
}

impl<T> AsPtr<T> for &T {
    fn as_ptr(&self) -> *const T {
        *self as *const T
    }
}

/// Convert to a mutable pointer.
pub trait AsMutPtr<T: ?Sized> {
    fn as_mut_ptr(&mut self) -> *mut T;
}

impl<T> AsMutPtr<T> for &mut T {
    fn as_mut_ptr(&mut self) -> *mut T {
        *self as *mut T
    }
}

/// A wrapper for pointers, holding either a raw address or some owned data
#[derive(Debug, Serialize, Deserialize, EnumIndex)]
pub enum Pointer<T> {
    /// Raw address
    Addr(usize),
    /// Owned data
    Data(T),
}

impl<T> AsPtr<T> for Pointer<T> {
    fn as_ptr(&self) -> *const T {
        match self {
            Pointer::Addr(addr) => *addr as *const T,
            Pointer::Data(data) => data as *const T,
        }
    }
}

impl<T> AsMutPtr<T> for Pointer<T> {
    fn as_mut_ptr(&mut self) -> *mut T {
        match self {
            Pointer::Addr(addr) => *addr as *mut T,
            Pointer::Data(data) => data as *mut T,
        }
    }
}

/// Result of a syscall, holding either a reference to some syscall result
/// or a specified value.
#[derive(Debug, Serialize, Deserialize, EnumIndex)]
pub enum SyscallResult {
    /// Reference to a syscall result
    Ref(Uuid),
    /// Specified value
    Value(u64),
}

/// Store syscall results to be used by later syscalls.
/// This is a fixed-size map if the `alloc` feature is not enabled.
#[cfg(not(feature = "alloc"))]
pub struct ResultContainer<const N: usize = DEFAULT_MAP_SIZE> {
    data: Map<Uuid, usize, N>,
}

/// Store syscall results to be used by later syscalls.
/// This is a `BTreeMap` if the `alloc` feature is enabled.
#[cfg(feature = "alloc")]
pub struct ResultContainer {
    data: Map<Uuid, usize>,
}

impl ResultContainer {
    /// Create a new result container.
    pub fn new() -> Self {
        Self { data: Map::new() }
    }

    /// Insert a result.
    pub fn insert(&mut self, key: Uuid, value: usize) {
        self.data.insert(key, value).unwrap();
    }

    /// Check if a result is present.
    pub fn contains_key(&self, key: &Uuid) -> bool {
        self.data.contains_key(key)
    }

    /// Get a result.
    pub fn get(&self, key: &Uuid) -> Option<usize> {
        self.data.get(&key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use enum_index::EnumIndex;

    #[test]
    fn test_pointer_index() {
        let addr: Pointer<i32> = Pointer::Addr(0x1234);
        let data = Pointer::Data(0x1234);

        assert_eq!(addr.enum_index(), 0);
        assert_eq!(data.enum_index(), 1);
    }
}
