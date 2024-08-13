#![no_std]

use core::marker::Sized;

use enum_index_derive::EnumIndex;
use serde::{Deserialize, Serialize};

/// Make a syscall with context
pub trait MakeSyscall {
    /// Syscall number
    const NR: i32;

    /// Call syscall
    fn call(&self) -> isize;
}

/// Make a syscall with context, mutable to receive data
pub trait MakeSyscallMut {
    /// Syscall number
    const NR: i32;

    /// Call syscall
    fn call(&mut self) -> isize;
}

/// Convert to a pointer
pub trait AsPtr<T: ?Sized> {
    fn as_ptr(&self) -> *const T;
}

impl<T> AsPtr<T> for &T {
    fn as_ptr(&self) -> *const T {
        *self as *const T
    }
}

/// Convert to a mutable pointer
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
