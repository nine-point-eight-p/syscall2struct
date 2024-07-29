/// Make a syscall with context
pub trait MakeSyscall {
    /// Syscall number
    const NR: usize;

    /// Call syscall
    fn call(&self) -> usize;
}

/// Make a syscall with context, mutable to receive data
pub trait MakeSyscallMut {
    /// Syscall number
    const NR: usize;

    /// Call syscall
    fn call(&mut self) -> usize;
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
pub trait AsMutPtr<T> {
    fn as_mut_ptr(&mut self) -> *mut T;
}

impl<T> AsMutPtr<T> for &mut T {
    fn as_mut_ptr(&mut self) -> *mut T {
        *self as *mut T
    }
}
