use core::{cmp::min, marker::PhantomData, mem::size_of};

use syscalls::{Error, Permissions};

use crate::{
    memory::VirtAddr,
    user::{
        error::check_arg_res,
        handle::Handle,
        process::{MemoryAccess, TypedMemoryAccess},
    },
};

use alloc::str;

use super::context::Context;

/// Helper object to operate output in 2 steps:
/// - fallible step: prepare pointer view
/// - infallible step: write output pointer
pub struct HandleOutputWriter {
    user_access: TypedMemoryAccess<Handle>,
}

impl HandleOutputWriter {
    // Create a new writer
    pub fn new(context: &Context, handle_out_ptr: usize) -> Result<Self, Error> {
        let thread = context.owner();
        let process = thread.process();

        let user_access = process.vm_access_typed(
            VirtAddr::new(handle_out_ptr as u64),
            Permissions::READ | Permissions::WRITE,
        )?;

        Ok(Self { user_access })
    }

    /// Write the handle output value
    pub fn set(&mut self, handle: Handle) {
        *self.user_access.get_mut() = handle;
    }
}

pub struct ListOutputWriter<T: Sized + Copy> {
    array_access: MemoryAccess,
    count_access: TypedMemoryAccess<usize>,
    _phantom: PhantomData<T>,
}

impl<T: Sized + Copy> ListOutputWriter<T> {
    // Create a new writer
    pub fn new(context: &Context, array_ptr: usize, count_ptr: usize) -> Result<Self, Error> {
        let thread = context.owner();
        let process = thread.process();

        let count_access = process.vm_access_typed(
            VirtAddr::new(count_ptr as u64),
            Permissions::READ | Permissions::WRITE,
        )?;

        let array_count = *count_access.get();
        let array_addr = VirtAddr::new(array_ptr as u64);

        let array_access = process.vm_access(
            array_addr..array_addr + (size_of::<T>() * array_count),
            Permissions::READ | Permissions::WRITE,
        )?;

        Ok(Self {
            array_access,
            count_access,
            _phantom: PhantomData,
        })
    }

    /// Fill the list
    pub fn fill(&mut self, source: &[T]) {
        let dest = self.array_access.get_slice_mut::<T>();
        let count = min(source.len(), dest.len());

        dest[0..count].copy_from_slice(&source[0..count]);
        *self.count_access.get_mut() = source.len();
    }
}

pub struct StringReader {
    access: MemoryAccess,
}

impl StringReader {
    pub fn new(context: &Context, ptr: usize, len: usize) -> Result<Self, Error> {
        let thread = context.owner();
        let process = thread.process();

        let message_range = VirtAddr::new(ptr as u64)..VirtAddr::new((ptr + len) as u64);
        let access = process.vm_access(message_range, Permissions::READ)?;

        Ok(Self { access })
    }

    /// Get the string
    pub fn str<'a>(&'a self) -> Result<&'a str, Error> {
        check_arg_res(str::from_utf8(self.access.get_slice::<u8>()))
    }
}
