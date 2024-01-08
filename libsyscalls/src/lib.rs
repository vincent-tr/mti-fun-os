#![no_std]

mod handle;
pub mod ipc;
pub mod listener;
mod logging;
pub mod memory_object;
pub mod process;
mod syscalls;
pub mod thread;

use core::{
    cmp::min,
    mem::{self, MaybeUninit},
};

pub use handle::*;
pub use logging::*;

use ::syscalls::SUCCESS;
pub use ::syscalls::{
    Error, HandleType, Message, Permissions, PortInfo, ProcessEvent, ProcessEventType, ProcessInfo,
    ThreadEvent, ThreadEventType, ThreadInfo, ThreadPriority, ThreadState,
};

pub type SyscallResult<T> = Result<T, Error>;

/// # Safety
///
/// Borrowing rules unchecked. Do right before syscalls only.
unsafe fn ref_ptr<T>(value: &T) -> usize {
    let ptr: *const T = value;
    ptr as usize
}

/// # Safety
///
/// Borrowing rules unchecked. Do right before syscalls only.
unsafe fn slice_ptr<T>(value: &[T]) -> usize {
    let ptr = value.as_ptr();
    ptr as usize
}

fn sysret_to_result(sysret: usize) -> SyscallResult<()> {
    match sysret {
        SUCCESS => Ok(()),
        err => Err(unsafe { mem::transmute(err) }),
    }
}

/// Manage a list in syscall
struct SyscallList<'a, T: Sized + Copy> {
    array: &'a mut [T],
    count: usize,
}

impl<'a, T: Sized + Copy> SyscallList<'a, T> {
    /// # Safety
    ///
    /// Structure must not be moved after creation, it must stay on stack
    pub unsafe fn new(array: &'a mut [T]) -> Self {
        let count = array.len();
        Self { array, count }
    }

    /// Get the array pointer argument for syscall
    ///
    /// # Safety
    /// No borrow rule are checked
    ///
    pub unsafe fn array_ptr_arg(&self) -> usize {
        self.array.as_ptr() as usize
    }

    /// Get the count pointer argument for syscall
    ///
    /// # Safety
    /// No borrow rule are checked
    ///
    pub unsafe fn count_ptr_arg(&self) -> usize {
        ref_ptr(&self.count)
    }

    /// Call after syscall to properly configure output slice
    pub fn finalize<'b>(&mut self) -> (&'b [T], usize) {
        let slice_count = min(self.count, self.array.len());

        // FIXME: indicate borrow checked that &[T] has same lifetime than 'array'
        //&self.array[0..slice_count]
        let ptr = self.array.as_ptr();
        let slice = unsafe { core::slice::from_raw_parts(ptr, slice_count) };

        (slice, self.count)
    }
}

struct SyscallOutPtr<T: Sized> {
    value: T,
}

impl<T> SyscallOutPtr<T> {
    pub const fn new() -> Self {
        let value: T = unsafe { MaybeUninit::uninit().assume_init() };
        Self { value }
    }

    pub unsafe fn ptr_arg(&self) -> usize {
        ref_ptr(&self.value)
    }

    pub fn take(self) -> T {
        self.value
    }
}

struct SyscallInStr<'a> {
    value: &'a [u8],
}

impl<'a> SyscallInStr<'a> {
    pub const fn new(value: &'a str) -> Self {
        Self {
            value: value.as_bytes(),
        }
    }

    pub unsafe fn ptr_arg(&self) -> usize {
        self.value.as_ptr() as usize
    }

    pub unsafe fn len_arg(&self) -> usize {
        self.value.len()
    }
}
